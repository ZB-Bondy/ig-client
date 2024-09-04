use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use rand::RngCore;

const NONCE_LENGTH: usize = 12;

pub struct Encryptor {
    cipher: Aes256Gcm,
}
impl Encryptor {
    pub fn new(key: &[u8]) -> Result<Self> {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce);
        let nonce = Nonce::from_slice(&nonce);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(general_purpose::STANDARD.encode(&result))
    }
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let ciphertext = general_purpose::STANDARD
            .decode(ciphertext)
            .context("Failed to decode base64")?;
        if ciphertext.len() < NONCE_LENGTH {
            anyhow::bail!("Ciphertext is too short");
        }

        let (nonce, ciphertext) = ciphertext.split_at(NONCE_LENGTH);
        let nonce = Nonce::from_slice(nonce);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

        String::from_utf8(plaintext).context("Failed to convert decrypted data to UTF-8")
    }
}

#[cfg(test)]
mod tests_encryptor {
    use super::*;

    fn generate_random_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    #[test]
    fn test_encryption_decryption() {
        let key = generate_random_key();
        let encryptor = Encryptor::new(&key).unwrap();

        let plaintext = "Hello, World!";
        let ciphertext = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_different_plaintexts() {
        let key = generate_random_key();
        let encryptor = Encryptor::new(&key).unwrap();

        let plaintexts = vec![
            "Short text",
            "A bit longer text with some numbers 12345",
            "Even longer text with special characters !@#$%^&*()",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam non felis et augue.",
        ];

        for plaintext in plaintexts {
            let ciphertext = encryptor.encrypt(plaintext).unwrap();
            let decrypted = encryptor.decrypt(&ciphertext).unwrap();
            assert_eq!(plaintext, decrypted);
        }
    }

    #[test]
    fn test_invalid_ciphertext() {
        let key = generate_random_key();
        let encryptor = Encryptor::new(&key).unwrap();

        let invalid_ciphertext = "This is not a valid ciphertext";
        assert!(encryptor.decrypt(invalid_ciphertext).is_err());
    }

    #[test]
    fn test_different_keys() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let encryptor1 = Encryptor::new(&key1).unwrap();
        let encryptor2 = Encryptor::new(&key2).unwrap();

        let plaintext = "Secret message";
        let ciphertext = encryptor1.encrypt(plaintext).unwrap();

        // Trying to decrypt with a different key should fail
        assert!(encryptor2.decrypt(&ciphertext).is_err());
    }
}
