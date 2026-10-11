//! Password-unlocked removable-media credential store for the interactive controller.
//! No passwords in command-line args, environment variables, or unencrypted files.
//! Protocol: Argon2id v=19, m=65536 KiB, t=3, p=1; XChaCha20-Poly1305,
//! unique random 192-bit nonce per write, authenticated record identity (AAD).
//! This is a candidate implementation; ship only after CI and threat-model review.
use anyhow::{Context, Result, bail, ensure};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{XChaCha20Poly1305, XNonce, KeyInit, aead::{Aead, Payload}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::{self, OpenOptions}, io::Write, path::{Path, PathBuf}, sync::OnceLock};
use zeroize::{Zeroize, Zeroizing};

struct Vault {
    dir: PathBuf,
    key: Zeroizing<[u8; 32]>,
}
static ACTIVE: OnceLock<Vault> = OnceLock::new();
const CHECK: &[u8] = b"AXY-portable-credential-vault/1";
const CHECK_ID: &str = "__vault_check_v1__";
const MAX_RECORD: usize = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Meta {
    version: u32,
    salt: [u8; 16],
}

pub(crate) fn enabled() -> bool { ACTIVE.get().is_some() }
pub(crate) fn root() -> Option<&'static Path> { ACTIVE.get().map(|v| v.dir.as_path()) }

/// Initializes *only* in an interactive controller process, not in services,
/// plugins, installers, background tasks, or elevated helper processes.
pub(crate) fn initialize() -> Result<()> {
    if std::env::var_os("AXY_OPENUUYC_PORTABLE").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Ok(());
    }
    let dir = PathBuf::from(std::env::var_os("AXY_OPENUUYC_VAULT_DIR")
        .context("portable vault folder not specified by launcher")?);
    ensure!(dir.is_absolute(), "portable vault path must be absolute");
    let display = dir.to_string_lossy();
    ensure!(!display.starts_with(r"\\"), "network portable vaults are not supported");
    fs::create_dir_all(&dir).context("create USB credential vault folder")?;
    let meta_path = dir.join("vault.json");
    let initial = !meta_path.exists();
    let meta = if initial {
        let meta = Meta { version: 1, salt: rand::random() };
        // Never overwrite an existing/partially initialized vault.
        let mut f = OpenOptions::new().write(true).create_new(true).open(&meta_path)?;
        f.write_all(&serde_json::to_vec(&meta)?)?;
        f.sync_all()?;
        meta
    } else {
        let bytes = fs::read(&meta_path)?;
        ensure!(bytes.len() < 4096, "credential vault header exceeds size bound");
        let meta: Meta = serde_json::from_slice(&bytes).context("invalid vault header")?;
        ensure!(meta.version == 1, "unsupported credential vault version");
        meta
    };
    let password = prompt_password("AXY 便携登录凭据库：请输入主密码")?;
    ensure!(password.chars().count() >= 12, "vault password must contain at least 12 characters");
    if initial {
        let confirmation = prompt_password("请再次输入便携库主密码（仅首次创建）")?;
        ensure!(password.as_str() == confirmation.as_str(), "password confirmation mismatch");
    }
    let params = Params::new(65536, 3, 1, Some(32)).context("Argon2id parameters")?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &meta.salt, &mut *key)
        .map_err(|_| anyhow::anyhow!("credential vault password derivation failed"))?;
    drop(password);
    let vault = Vault { dir, key };
    let check_path = vault.dir.join("vault.check");
    if initial {
        ensure!(!check_path.exists(), "existing vault marker; refusing to overwrite");
        vault.write_path(&check_path, CHECK_ID, CHECK)?;
    } else {
        let decrypted = vault.read_path(&check_path, CHECK_ID)?
            .context("missing vault marker, refusing recovery by creating an empty vault")?;
        ensure!(decrypted == CHECK, "vault password incorrect or encrypted data damaged");
    }
    ACTIVE.set(vault).map_err(|_| anyhow::anyhow!("portable vault already initialized"))?;
    Ok(())
}

fn prompt_password(message: &str) -> Result<Zeroizing<String>> {
    use windows::{core::PCWSTR, Win32::Security::Credentials::*};
    let _ = message; // Standard Windows password dialog; message is intentionally not logged.
    let mut user = [0u16; 256];
    let mut password = Zeroizing::new([0u16; 256]);
    let status = unsafe {
        CredUIPromptForCredentialsW(
            None, windows::core::w!("AXY Portable OpenUUYC"), None, 0,
            &mut user, &mut *password, None,
            CREDUI_FLAGS_GENERIC_CREDENTIALS |
            CREDUI_FLAGS_PASSWORD_ONLY_OK |
            CREDUI_FLAGS_ALWAYS_SHOW_UI |
            CREDUI_FLAGS_DO_NOT_PERSIST
        )
    };
    ensure!(status.0 == 0, "vault unlock cancelled or Windows credential dialog unavailable");
    let length = password.iter().position(|c| *c == 0).unwrap_or(password.len());
    let value = Zeroizing::new(String::from_utf16(&password[..length])
        .context("invalid password characters")?);
    password.zeroize();
    Ok(value)
}

impl Vault {
    fn file(&self, service: &str, account: &str) -> PathBuf {
        let digest = Sha256::digest(format!("{service}\0{account}"));
        self.dir.join(format!("{digest:x}.enc"))
    }
    fn encrypt(&self, id: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        ensure!(bytes.len() <= MAX_RECORD, "credential record too large");
        let nonce: [u8; 24] = rand::random();
        let aead = XChaCha20Poly1305::new((&*self.key).into());
        let ciphertext = aead.encrypt(XNonce::from_slice(&nonce),
            Payload { msg: bytes, aad: id.as_bytes() })
            .map_err(|_| anyhow::anyhow!("credential vault encryption failed"))?;
        let mut output = Vec::with_capacity(24 + ciphertext.len());
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }
    fn decrypt(&self, id: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        ensure!(bytes.len() >= 40 && bytes.len() <= MAX_RECORD + 40,
            "invalid encrypted record length");
        let aead = XChaCha20Poly1305::new((&*self.key).into());
        aead.decrypt(XNonce::from_slice(&bytes[..24]),
            Payload { msg: &bytes[24..], aad: id.as_bytes() })
            .map_err(|_| anyhow::anyhow!("wrong password, corrupt credential or swapped record"))
    }
    fn read_path(&self, path: &Path, id: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(path) {
            Ok(bytes) => self.decrypt(id, &bytes).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn write_path(&self, path: &Path, id: &str, bytes: &[u8]) -> Result<()> {
        let sealed = self.encrypt(id, bytes)?;
        let temporary = self.dir.join(format!(".{}.tmp", uuid::Uuid::new_v4().simple()));
        let result = (|| -> Result<()> {
            let mut f = OpenOptions::new().write(true).create_new(true).open(&temporary)?;
            f.write_all(&sealed)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&temporary, path).context("atomically replace encrypted credential")?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(&temporary); }
        result
    }
}

pub(crate) fn read(service: &str, account: &str) -> Result<Option<Vec<u8>>> {
    let v = ACTIVE.get().context("portable vault was not unlocked")?;
    v.read_path(&v.file(service, account), &format!("{service}\0{account}"))
}
pub(crate) fn write(service: &str, account: &str, bytes: Option<&[u8]>) -> Result<()> {
    let v = ACTIVE.get().context("portable vault was not unlocked")?;
    let path = v.file(service, account);
    match bytes {
        Some(bytes) => v.write_path(&path, &format!("{service}\0{account}"), bytes),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
