//! `src/main/secrets.ts`: encrypts and decrypts secret run properties. Electron Grace asked Electron's `safeStorage`
//! for it, whose key lives in Chromium's `Local State`; GPUI Grace asks the platform directly (migration guide R5.3):
//! DPAPI for the current user on Windows, and on Linux the desktop's Secret Service, which is not wired yet, so a
//! Linux box reports itself unavailable and the settings refuse secret properties with a notice.
//!
//! Old `safeStorage` values are not read (R5.4, "No backward compatibility"): their names and the plain values stay,
//! the secret values are entered or imported again.
//!
//! The stored format is Electron Grace's: the ciphertext as base64 text, in the same JSON files. What the bytes mean
//! is the platform's, and differs from `safeStorage`'s.
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::fmt;

/// Why a secret could not be encrypted. TypeScript's `encrypt` throws; this is the same case as a value, and it
/// never holds the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretError {
    /// There is no secret storage on this machine.
    Unavailable,
    /// The storage is there and refused this one.
    Failed,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "secret storage is not available",
            Self::Failed => "the secret could not be encrypted",
        })
    }
}

impl std::error::Error for SecretError {}

/// Encrypts and decrypts secret run properties. A seam, so tests need no operating system storage.
pub trait SecretBox: Send + Sync {
    fn available(&self) -> bool;
    /// Base64 ciphertext.
    fn encrypt(&self, plain: &str) -> Result<String, SecretError>;
    /// The plain text, or `None` if the ciphertext cannot be read. Never panics.
    fn decrypt(&self, cipher: &str) -> Option<String>;
}

/// What the platform gives the box: protect bytes for the current user, and take them back. TypeScript's
/// `SafeStorageLike`; bytes rather than strings, since that is what DPAPI speaks.
pub trait Protector: Send + Sync {
    fn available(&self) -> bool;
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, SecretError>;
    /// `None` for anything this protector did not produce, for this user.
    fn unprotect(&self, blob: &[u8]) -> Option<Vec<u8>>;
}

/// A box over a platform's [`Protector`]: the ciphertext is the base64 of what it produces
/// (TypeScript's `safeStorageBox`).
pub struct ProtectedBox<P> {
    protector: P,
}

impl<P: Protector> ProtectedBox<P> {
    pub fn new(protector: P) -> Self {
        Self { protector }
    }
}

impl<P: Protector> SecretBox for ProtectedBox<P> {
    fn available(&self) -> bool {
        self.protector.available()
    }

    fn encrypt(&self, plain: &str) -> Result<String, SecretError> {
        Ok(STANDARD.encode(self.protector.protect(plain.as_bytes())?))
    }

    fn decrypt(&self, cipher: &str) -> Option<String> {
        let blob = STANDARD.decode(cipher).ok()?;
        String::from_utf8(self.protector.unprotect(&blob)?).ok()
    }
}

/// The real box of this platform. TypeScript's `safeStorageBox()`.
pub fn os_secret_box() -> ProtectedBox<OsProtector> {
    ProtectedBox::new(OsProtector)
}

pub use os::OsProtector;

/// For tests only: base64 of a prefixed copy, so ciphertext differs from plain text and corrupt input is detectable.
/// TypeScript's `fakeSecretBox(available = true)`.
#[derive(Clone, Copy, Debug)]
pub struct FakeSecretBox {
    available: bool,
}

const FAKE_PREFIX: &str = "fake:";

impl FakeSecretBox {
    pub fn new(available: bool) -> Self {
        Self { available }
    }
}

impl Default for FakeSecretBox {
    fn default() -> Self {
        Self::new(true)
    }
}

impl SecretBox for FakeSecretBox {
    fn available(&self) -> bool {
        self.available
    }

    // Like the TypeScript fake, encrypts even when it reports itself unavailable.
    fn encrypt(&self, plain: &str) -> Result<String, SecretError> {
        Ok(STANDARD.encode(format!("{FAKE_PREFIX}{plain}")))
    }

    fn decrypt(&self, cipher: &str) -> Option<String> {
        let bytes = STANDARD.decode(cipher).ok()?;
        let text = String::from_utf8_lossy(&bytes);
        text.strip_prefix(FAKE_PREFIX).map(str::to_string)
    }
}

/// What DPAPI protects: a version byte, then the text. DPAPI then never sees an empty input (a secret may be
/// empty), and a later format can tell itself from this one.
#[cfg(any(windows, test))]
const FRAME_VERSION: u8 = 1;

#[cfg(any(windows, test))]
fn frame(plain: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(plain.len() + 1);
    framed.push(FRAME_VERSION);
    framed.extend_from_slice(plain);
    framed
}

/// The text of a framed blob, `None` for anything else.
#[cfg(any(windows, test))]
fn unframe(framed: &[u8]) -> Option<&[u8]> {
    match framed.split_first() {
        Some((&FRAME_VERSION, text)) => Some(text),
        _ => None,
    }
}

/// Windows: DPAPI, `CryptProtectData` and `CryptUnprotectData` for the current user, no extra entropy and no prompt.
#[cfg(windows)]
mod os {
    use super::{Protector, SecretError, frame, unframe};
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };
    use windows::core::PCWSTR;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct OsProtector;

    /// Copies a buffer DPAPI allocated, wipes it and frees it.
    ///
    /// # Safety
    /// `blob` must be an output of a successful `CryptProtectData` or `CryptUnprotectData`, not yet freed.
    unsafe fn take(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let len = blob.cbData as usize;
        // SAFETY: per the contract, `pbData` points at `cbData` readable and writable bytes from `LocalAlloc`.
        unsafe {
            let bytes = std::slice::from_raw_parts(blob.pbData, len).to_vec();
            std::ptr::write_bytes(blob.pbData, 0, len);
            let _ = LocalFree(Some(HLOCAL(blob.pbData.cast())));
            bytes
        }
    }

    fn dpapi_protect(data: &[u8]) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(data.len()).ok()?,
            pbData: data.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        // SAFETY: `input` points at `data`, which outlives the call and which DPAPI only reads. On success `output`
        // is a buffer DPAPI allocated, which `take` consumes.
        unsafe {
            CryptProtectData(
                &input,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .ok()?;
            Some(take(output))
        }
    }

    fn dpapi_unprotect(blob: &[u8]) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(blob.len()).ok()?,
            pbData: blob.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        // SAFETY: as in `dpapi_protect`.
        unsafe {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).ok()?;
            Some(take(output))
        }
    }

    impl Protector for OsProtector {
        /// Whether DPAPI answers for this user, asked once.
        fn available(&self) -> bool {
            static AVAILABLE: OnceLock<bool> = OnceLock::new();
            *AVAILABLE.get_or_init(|| dpapi_protect(&frame(b"probe")).is_some())
        }

        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, SecretError> {
            dpapi_protect(&frame(plain)).ok_or(SecretError::Failed)
        }

        fn unprotect(&self, blob: &[u8]) -> Option<Vec<u8>> {
            unframe(&dpapi_unprotect(blob)?).map(<[u8]>::to_vec)
        }
    }
}

/// Linux: no secret storage yet, so the box reports itself unavailable and the settings refuse secret properties with
/// a notice (migration guide R5.3). Follow-up: the desktop's Secret Service, over D-Bus. `oo7` is already in
/// `Cargo.lock` through GPUI's Linux platform crate; whether to take it is that follow-up's decision.
#[cfg(not(windows))]
mod os {
    use super::{Protector, SecretError};

    #[derive(Clone, Copy, Debug, Default)]
    pub struct OsProtector;

    impl Protector for OsProtector {
        fn available(&self) -> bool {
            false
        }

        fn protect(&self, _plain: &[u8]) -> Result<Vec<u8>, SecretError> {
            Err(SecretError::Unavailable)
        }

        fn unprotect(&self, _blob: &[u8]) -> Option<Vec<u8>> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base64_of(text: &str) -> String {
        STANDARD.encode(text)
    }

    // Stands in for the platform's protector (Electron's safeStorage in the TypeScript test): `unprotect` gives
    // `None` for anything it did not protect, as `decryptString` throws.
    #[derive(Clone, Copy)]
    struct Stub {
        available: bool,
        broken: bool,
    }

    fn storage(available: bool) -> Stub {
        Stub {
            available,
            broken: false,
        }
    }

    impl Protector for Stub {
        fn available(&self) -> bool {
            self.available
        }

        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, SecretError> {
            Ok([b"enc:".as_slice(), plain].concat())
        }

        fn unprotect(&self, blob: &[u8]) -> Option<Vec<u8>> {
            if self.broken {
                return None; // the TypeScript's `decryptString` that throws "boom"
            }
            blob.strip_prefix(b"enc:").map(<[u8]>::to_vec)
        }
    }

    mod fake_secret_box {
        use super::*;

        #[test]
        fn round_trips_and_rejects_corrupt_input() {
            let secret_box = FakeSecretBox::default();
            let cipher = secret_box.encrypt("s3cret").unwrap();
            assert!(!cipher.contains("s3cret"));
            // the format of the TypeScript fake: `Buffer.from("fake:s3cret").toString("base64")`
            assert_eq!(cipher, "ZmFrZTpzM2NyZXQ=");
            assert_eq!(cipher, base64_of("fake:s3cret"));
            assert_eq!(secret_box.decrypt(&cipher).as_deref(), Some("s3cret"));
            let empty = secret_box.encrypt("").unwrap();
            assert_eq!(secret_box.decrypt(&empty).as_deref(), Some(""));
            assert_eq!(secret_box.decrypt(""), None);
            assert_eq!(secret_box.decrypt("!!not base64!!"), None);
            assert_eq!(secret_box.decrypt(&base64_of("garbage")), None);
        }

        #[test]
        fn reports_whether_it_is_available() {
            assert!(FakeSecretBox::default().available());
            assert!(FakeSecretBox::new(true).available());
            assert!(!FakeSecretBox::new(false).available());
        }
    }

    // The TypeScript's `describe("safeStorageBox")`: the box over a platform's protector.
    mod protected_box {
        use super::*;

        #[test]
        fn stores_the_base64_of_the_encrypted_string_and_reads_it_back() {
            let secret_box = ProtectedBox::new(storage(true));
            let cipher = secret_box.encrypt("s3cret").unwrap();
            assert_eq!(cipher, base64_of("enc:s3cret"));
            assert_eq!(secret_box.decrypt(&cipher).as_deref(), Some("s3cret"));
            let empty = secret_box.encrypt("").unwrap();
            assert_eq!(secret_box.decrypt(&empty).as_deref(), Some(""));
        }

        #[test]
        fn reports_whether_encryption_is_available() {
            assert!(ProtectedBox::new(storage(true)).available());
            assert!(!ProtectedBox::new(storage(false)).available());
        }

        #[test]
        fn decrypt_never_throws() {
            let secret_box = ProtectedBox::new(storage(true));
            assert_eq!(secret_box.decrypt("!!not base64!!"), None);
            assert_eq!(secret_box.decrypt(""), None);
            assert_eq!(secret_box.decrypt(&base64_of("garbage")), None);
            let broken = ProtectedBox::new(Stub {
                available: true,
                broken: true,
            });
            assert_eq!(broken.decrypt(&secret_box.encrypt("s3cret").unwrap()), None);
        }

        // The TypeScript's "loads without Electron" checks that importing secrets.ts in plain Node does not need
        // Electron's main-process API: there is no Electron here, so there is nothing to port (n/a).

        // Not in the TypeScript tests: a stored text that is not UTF-8 is unreadable, not garbled.
        #[test]
        fn a_blob_that_is_not_text_is_unreadable() {
            let secret_box = ProtectedBox::new(storage(true));
            let cipher = STANDARD.encode([b"enc:".as_slice(), &[0xFF, 0xFE]].concat());
            assert_eq!(secret_box.decrypt(&cipher), None);
        }

        // Not in the TypeScript tests: `encrypt` throws there when there is no storage.
        #[test]
        fn encrypt_says_when_there_is_no_storage() {
            struct Missing;
            impl Protector for Missing {
                fn available(&self) -> bool {
                    false
                }
                fn protect(&self, _plain: &[u8]) -> Result<Vec<u8>, SecretError> {
                    Err(SecretError::Unavailable)
                }
                fn unprotect(&self, _blob: &[u8]) -> Option<Vec<u8>> {
                    None
                }
            }
            assert_eq!(
                ProtectedBox::new(Missing).encrypt("s3cret"),
                Err(SecretError::Unavailable)
            );
        }
    }

    // One test on each platform (guide R5.3): what the system box does.
    mod the_system_box {
        use super::*;

        #[cfg(not(windows))]
        #[test]
        fn is_unavailable_without_a_secret_service() {
            let secret_box = os_secret_box();
            assert!(!secret_box.available());
            assert_eq!(secret_box.encrypt("s3cret"), Err(SecretError::Unavailable));
            assert_eq!(secret_box.decrypt(&base64_of("anything")), None);
        }

        #[cfg(windows)]
        #[test]
        fn is_dpapi_for_the_current_user() {
            let secret_box = os_secret_box();
            assert!(secret_box.available());
            let cipher = secret_box.encrypt("s3cret").unwrap();
            assert!(!cipher.contains("s3cret"));
            assert_eq!(secret_box.decrypt(&cipher).as_deref(), Some("s3cret"));
            let empty = secret_box.encrypt("").unwrap();
            assert_eq!(secret_box.decrypt(&empty).as_deref(), Some(""));
            let unicode = secret_box.encrypt("pässwörd €😀").unwrap();
            assert_eq!(secret_box.decrypt(&unicode).as_deref(), Some("pässwörd €😀"));
            // what another box wrote, or garbage, is unreadable
            assert_eq!(
                secret_box.decrypt(&FakeSecretBox::default().encrypt("s3cret").unwrap()),
                None
            );
            assert_eq!(secret_box.decrypt("!!not base64!!"), None);
            assert_eq!(secret_box.decrypt(""), None);
        }
    }

    mod frame {
        use super::*;

        #[test]
        fn puts_a_version_byte_before_the_text_so_it_is_never_empty() {
            assert_eq!(frame(b""), [1]);
            assert_eq!(frame(b"ab"), [1, b'a', b'b']);
            assert_eq!(unframe(&frame(b"ab")), Some(b"ab".as_slice()));
            assert_eq!(unframe(&frame(b"")), Some(b"".as_slice()));
        }

        #[test]
        fn refuses_anything_else() {
            assert_eq!(unframe(b""), None);
            assert_eq!(unframe(&[2, b'a']), None);
            assert_eq!(unframe(b"ab"), None);
        }
    }
}
