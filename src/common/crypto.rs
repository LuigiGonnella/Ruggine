// Common cryptographic utilities and types shared between client and server
use argon2::{Argon2, password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}};
use rand::{RngCore, rngs::OsRng};
use serde::{Serialize, Deserialize};
use ring::aead::{self, AES_256_GCM, LessSafeKey, UnboundKey, Nonce, NONCE_LEN};
use ring::error::Unspecified;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedMessage {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub sender_id: String,
    pub recipient_id: Option<String>,
    pub group_id: Option<String>,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

pub struct CryptoManager;

impl CryptoManager {
    pub fn hash_password(password: &str, _salt_length: usize) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        password_hash.to_string()
    }

    pub fn verify_password(hash: &str, password: &str) -> bool {
        let parsed_hash = PasswordHash::new(hash).unwrap();
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    }

    /// Generates a 256-bit key from a password using PBKDF2
    pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32], Unspecified> {
        use ring::pbkdf2;
        let mut key = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(100_000).unwrap(),
            salt,
            password.as_bytes(),
            &mut key,
        );
        Ok(key)
    }

    /// Generates a master key for the chat system
    pub fn generate_master_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Encrypts a message using AES-256-GCM
    pub fn encrypt_message(plaintext: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), Unspecified> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
        let key = LessSafeKey::new(unbound_key);
        
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        
        let mut ciphertext = plaintext.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut ciphertext)?;
        
        Ok((ciphertext, nonce_bytes.to_vec()))
    }

    /// Decrypts a message using AES-256-GCM
    pub fn decrypt_message(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<String, Unspecified> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
        let key = LessSafeKey::new(unbound_key);
        
        let nonce_array: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| Unspecified)?;
        let nonce = Nonce::assume_unique_for_key(nonce_array);
        
        let mut ciphertext_copy = ciphertext.to_vec();
        let plaintext = key.open_in_place(nonce, aead::Aad::empty(), &mut ciphertext_copy)?;
        
        String::from_utf8(plaintext.to_vec()).map_err(|_| Unspecified)
    }

    /// Generates a chat-specific key based on participant IDs
    pub fn generate_chat_key(participants: &[String], master_key: &[u8; 32]) -> [u8; 32] {
        use ring::digest;
        
        // Sort participants to ensure consistent key generation
        let mut sorted_participants = participants.to_vec();
        sorted_participants.sort();
        
        // Create input for key derivation
        let mut input = master_key.to_vec();
        for participant in sorted_participants {
            input.extend_from_slice(participant.as_bytes());
        }
        
        // Use SHA-256 to derive the chat key
        let digest = digest::digest(&digest::SHA256, &input);
        let mut chat_key = [0u8; 32];
        chat_key.copy_from_slice(digest.as_ref());
        
        chat_key
    }

    pub fn generate_nonce(length: usize) -> Vec<u8> {
        let mut nonce = vec![0u8; length];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    /// Parse a 64-char hex string into a 32-byte key
    pub fn parse_master_key_hex(key_hex: &str) -> Option<[u8; 32]> {
        if key_hex.len() != 64 { return None; }
        let mut key = [0u8; 32];
        for i in 0..32 {
            let byte_str = &key_hex[i*2..(i*2)+2];
            match u8::from_str_radix(byte_str, 16) {
                Ok(b) => key[i] = b,
                Err(_) => return None,
            }
        }
        Some(key)
    }

    /// Load master key from ENCRYPTION_MASTER_KEY env (via dotenv). Returns None if missing/invalid.
    pub fn load_master_key_from_env() -> Option<[u8; 32]> {
        // First try dotenv to populate environment
        let _ = dotenvy::dotenv();
        if let Ok(key_hex) = env::var("ENCRYPTION_MASTER_KEY") {
            if let Some(k) = Self::parse_master_key_hex(&key_hex) {
                return Some(k);
            } else {
                println!("[CRYPTO] ENCRYPTION_MASTER_KEY present but not valid hex");
            }
        }

        // Fallback: try to read common .env file locations to extract the key manually.
    let mut candidates = vec![
            std::path::PathBuf::from(".env"),
            std::path::PathBuf::from("../.env"),
        ];
        // CARGO_MANIFEST_DIR is set during build; try to check it at runtime as well
        if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
            candidates.push(std::path::PathBuf::from(manifest).join(".env"));
        }

        for path in candidates {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    for line in contents.lines() {
                        let l = line.trim();
                        if l.starts_with('#') || l.is_empty() { continue; }
                        if let Some((k, v)) = l.split_once('=') {
                            if k.trim() == "ENCRYPTION_MASTER_KEY" {
                                let value = v.trim().trim_matches('"');
                                if let Some(parsed) = Self::parse_master_key_hex(value) {
                                    println!("[CRYPTO] Loaded ENCRYPTION_MASTER_KEY from file: {}", path.display());
                                    return Some(parsed);
                                } else {
                                    println!("[CRYPTO] ENCRYPTION_MASTER_KEY in {} is not valid hex", path.display());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

// Add more cryptographic utilities as needed for features (e.g., key exchange, signatures)
