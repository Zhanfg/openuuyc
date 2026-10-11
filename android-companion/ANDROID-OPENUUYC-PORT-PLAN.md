# Android native OpenUUYC controller roadmap

This document is an implementation boundary, **not** a claim that Android remote display already works. The Android companion APK under `android-companion/` is a USB management shell and does not yet implement the remote desktop protocol.

## Target

- Android 8.0+ minimum; primary test devices ARM64 Android 16/17.
- **Controller only**: initiate authorized remote sessions; never advertise the Android phone as a remotely controllable host.
- Compatible with `AXY-DevStack/` on removable USB drives through Android Storage Access Framework (SAF). No `MANAGE_EXTERNAL_STORAGE`, accessibility service, root or experimental virtual display drivers.
- A stable Android-controller client identifier, **distinct from Windows host-registration identities**, plus user-authorized account login.
- Client credentials are never serialized in Android plaintext prefs or logs. A cross-device removable-media vault requires password-unlocked Argon2id/XChaCha20-Poly1305 or an audited KDBX library and explicit unlock; Android Keystore-only secrets are not portable to other phones.
- The service may reject the carried login token. Never bypass device authorization, login or remote-user approval.

## Repository findings

- `src/account/api`, `src/account/client`, `src/session`, `src/transport`, `src/protocol` contain candidate Rust logic to refactor into a platform-independent controller core.
- `src/platform/windows`, `src/application/app` (egui/DirectX), `src/media` encode/decode and Windows Credential Manager cannot be directly cross-compiled.
- Existing Windows `NativeIdentity` schema stores real `system_id` and `machine_guid`; the Android client must define a separate protocol-supported identity with stable persistence, not fabricate arbitrary hardware fingerprints.

## Milestones / acceptance criteria

1. **Protocol extraction**: compile a headless Rust client with Android NDK for `aarch64-linux-android`; split Windows-only imports behind `cfg(windows)` and Android interface behind `cfg(target_os = "android")`. Unit tests for message codec, request signing and connection cancellation.
2. **Auth and identity**: Android controller registration with service-accepted platform semantics; explicit QR/SMS login; encrypted logout/session rotation and rejection handling. Never automatically reuse a Windows host's device identity.
3. **JNI & app**: `libopenuuyc_controller.so` with typed JNI requests/responses; Kotlin Material3 screen lists authorized devices. No background receiver.
4. **Video**: `MediaCodec` decode H.264 first. Add HEVC/AV1 via capability negotiation, `Surface` compositing, rotation, color handling and frame pacing.
5. **Remote input**: map Android touch/pointer/keyboard to controller protocol input with session-bound authorization; no local AccessibilityService.
6. **Audio, clipboard and file**: Android AudioTrack/audio focus and clipboard opt-in; files through SAF with explicit URI grants.
7. **Network**: P2P/relay negotiation, live latency telemetry, bounded reconnect backoff and background lifecycle handling. Bettbox remains a separate VPN app; no unmanaged force-routing or token transfer.
8. **QA**: test both same-account and assistance flows; interruption/USB unplug, password retry, revoke, invalid token, multiple display, high-latency 4G, battery drain, screen lock, and Android 16/17 compatibility.

## Repository work hygiene

- Track Android native controller changes separately from Windows USB prototype branch, PR #1.
- The Android companion shell is PR #2 and may be tested on phone independently.
- No binary redistribution of modified OpenUUYC until project's restrictive LICENSE and third-party notices have been reviewed.
