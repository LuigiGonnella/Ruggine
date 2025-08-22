// Common cryptographic utilities and types shared between client and server
use argon2::{self, Config as Argon2Config};
use rand::{RngCore, rngs::OsRng};
use base64::{encode, decode};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedMessage {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub sender_id: i64,
    pub recipient_id: Option<i64>,
    pub group_id: Option<i64>,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

pub struct CryptoManager;

impl CryptoManager {
    pub fn hash_password(password: &str, salt_length: usize) -> String {
        let mut salt = vec![0u8; salt_length];
        OsRng.fill_bytes(&mut salt);
        let config = Argon2Config::default();
        let hash = argon2::hash_encoded(password.as_bytes(), &salt, &config).unwrap();
        hash
    }

    pub fn verify_password(hash: &str, password: &str) -> bool {
        argon2::verify_encoded(hash, password.as_bytes()).unwrap_or(false)
    }

    pub fn encrypt_message(plaintext: &str, key: &[u8], nonce: &[u8]) -> Vec<u8> {
        // Placeholder: implement with a proper symmetric encryption (e.g., AES-GCM)
        // For now, just base64 encode for demonstration
        encode(plaintext).as_bytes().to_vec()
    }

    pub fn decrypt_message(ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> String {
        // Placeholder: implement with a proper symmetric decryption
        // For now, just base64 decode for demonstration
        let decoded = decode(ciphertext).unwrap_or_default();
        String::from_utf8(decoded).unwrap_or_default()
    }

    pub fn generate_nonce(length: usize) -> Vec<u8> {
        let mut nonce = vec![0u8; length];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

// Add more cryptographic utilities as needed for features (e.g., key exchange, signatures)
