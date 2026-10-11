//! Shared account records in portable and explicitly installed modes.
use crate::platform::windows::host_service::vault;
use keyring::{Entry, Error};

pub(crate) struct SecretEntry {
    local: Entry,
    key: String,
    service: String,
    account: String,
}
fn failure(error: anyhow::Error) -> Error {
    Error::PlatformFailure(Box::new(std::io::Error::other(error.to_string())))
}
impl SecretEntry {
    fn removable(&self) -> bool {
        super::portable_vault::enabled()
            && (self.service == "com.openuuyc.session"
                || self.service.starts_with("com.openuuyc.assist."))
    }
    pub fn new(service: &str, account: &str) -> Result<Self, Error> {
        Ok(Self {
            local: Entry::new(service, account)?,
            key: vault::key(service, account),
            service: service.to_owned(),
            account: account.to_owned(),
        })
    }
    pub fn get_secret(&self) -> Result<Vec<u8>, Error> {
        if self.removable() {
            match super::portable_vault::read(&self.service, &self.account).map_err(failure)? {
                Some(bytes) => return Ok(bytes),
                None => {
                    // Never import a physical machine identity or an arbitrary
                    // other Windows user's credentials into the removable vault.
                    // Explicit one-time import is available for login sessions only.
                    let import = self.service == "com.openuuyc.session"
                        && self.account == "default-nrd-login"
                        && std::env::var_os("AXY_OPENUUYC_IMPORT_SESSION").as_deref()
                            == Some(std::ffi::OsStr::new("1"));
                    if !import {
                        return Err(Error::NoEntry);
                    }
                    let existing = self.local.get_secret()?;
                    super::portable_vault::write(&self.service, &self.account, Some(&existing))
                        .map_err(failure)?;
                    return Ok(existing);
                }
            }
        }
        if vault::applies().map_err(failure)? {
            vault::read(&self.key)
                .map_err(failure)?
                .ok_or(Error::NoEntry)
        } else {
            self.local.get_secret()
        }
    }
    pub fn set_secret(&self, bytes: &[u8]) -> Result<(), Error> {
        if self.removable() {
            return super::portable_vault::write(&self.service, &self.account, Some(bytes))
                .map_err(failure);
        }
        if vault::applies().map_err(failure)? {
            vault::write(&self.key, Some(bytes)).map_err(failure)
        } else {
            self.local.set_secret(bytes)
        }
    }
    pub fn delete_credential(&self) -> Result<(), Error> {
        if self.removable() {
            return super::portable_vault::write(&self.service, &self.account, None)
                .map_err(failure);
        }
        if vault::applies().map_err(failure)? {
            vault::write(&self.key, None).map_err(failure)
        } else {
            self.local.delete_credential()
        }
    }
    pub(crate) fn enroll(&self) -> anyhow::Result<()> {
        match self.local.get_secret() {
            Ok(bytes) => vault::write(&self.key, Some(&bytes)),
            Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    pub(crate) fn restore_portable(&self) -> anyhow::Result<()> {
        match vault::read(&self.key)? {
            Some(bytes) => self.local.set_secret(&bytes)?,
            None => match self.local.delete_credential() {
                Ok(()) | Err(Error::NoEntry) => (),
                Err(e) => return Err(e.into()),
            },
        }
        Ok(())
    }
    pub(crate) fn clear_portable(&self) -> anyhow::Result<()> {
        match self.local.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
