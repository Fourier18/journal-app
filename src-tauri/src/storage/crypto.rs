use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use std::io::{Read, Write};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::StorageError;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 32;
/// 64 MiB memory, 3 iterations, 1 thread — OWASP recommended minimum for interactive logins.
const ARGON2_MEM_KB: u32 = 65536;
const ARGON2_ITERS: u32 = 3;
const ARGON2_PARA: u32 = 1;

/// In-memory vault key. Zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultKey(pub [u8; KEY_LEN]);

impl VaultKey {
    /// Derive a vault key from a master password and a stored salt using argon2id.
    pub fn derive(password: &str, salt: &[u8]) -> Result<Self, StorageError> {
        let params = Params::new(ARGON2_MEM_KB, ARGON2_ITERS, ARGON2_PARA, Some(KEY_LEN))
            .map_err(|e| StorageError::Crypto(e.to_string()))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = [0u8; KEY_LEN];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| StorageError::Crypto(e.to_string()))?;
        Ok(VaultKey(key))
    }

    /// Encode key bytes as a URL-safe base64 string for use as an age passphrase.
    fn as_passphrase(&self) -> age::secrecy::SecretString {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        age::secrecy::SecretString::new(URL_SAFE_NO_PAD.encode(self.0))
    }
}

/// Generate a fresh random salt.
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Encrypt plaintext bytes with age passphrase mode, key derived from VaultKey.
pub fn encrypt(plaintext: &[u8], key: &VaultKey) -> Result<Vec<u8>, StorageError> {
    let passphrase = key.as_passphrase();
    let encryptor = age::Encryptor::with_user_passphrase(passphrase);
    let mut ciphertext = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut ciphertext)
        .map_err(|e| StorageError::Crypto(e.to_string()))?;
    writer
        .write_all(plaintext)
        .map_err(|e| StorageError::Crypto(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| StorageError::Crypto(e.to_string()))?;
    Ok(ciphertext)
}

/// Decrypt an age-encrypted blob using the VaultKey.
pub fn decrypt(ciphertext: &[u8], key: &VaultKey) -> Result<Vec<u8>, StorageError> {
    let passphrase = key.as_passphrase();
    let cursor = std::io::Cursor::new(ciphertext);
    let decryptor = match age::Decryptor::new(cursor)
        .map_err(|e| StorageError::Crypto(e.to_string()))?
    {
        age::Decryptor::Passphrase(d) => d,
        _ => {
            return Err(StorageError::Crypto(
                "Expected passphrase-encrypted file".into(),
            ))
        }
    };
    let mut plaintext = Vec::new();
    let mut reader = decryptor
        .decrypt(&passphrase, None)
        .map_err(|_| StorageError::WrongPassword)?;
    reader
        .read_to_end(&mut plaintext)
        .map_err(|e| StorageError::Crypto(e.to_string()))?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let salt = generate_salt();
        let key = VaultKey::derive("my-secret-password", &salt).unwrap();
        let plaintext = b"Hello, encrypted journal!";

        let ciphertext = encrypt(plaintext, &key).unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = decrypt(&ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_password_fails() {
        let salt = generate_salt();
        let key = VaultKey::derive("correct-password", &salt).unwrap();
        let wrong_key = VaultKey::derive("wrong-password", &salt).unwrap();

        let ciphertext = encrypt(b"secret", &key).unwrap();
        let result = decrypt(&ciphertext, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let k1 = VaultKey::derive("password", &salt1).unwrap();
        let k2 = VaultKey::derive("password", &salt2).unwrap();
        assert_ne!(k1.0, k2.0);
    }
}
