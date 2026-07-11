use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use std::io::Cursor;

// PBKDF2를 사용해 256비트 대칭키 파생 (10,000회 반복)
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 10_000, &mut key);
    key
}

// AES-256-GCM 암호화
pub fn encrypt_aes_gcm(data: &[u8], key: &[u8; 32], iv: &[u8; 12]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Key init failed: {}", e))?;
    let nonce = Nonce::from_slice(iv);
    
    cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Encryption failed: {}", e))
}

// AES-256-GCM 복호화
pub fn decrypt_aes_gcm(encrypted_data: &[u8], key: &[u8; 32], iv: &[u8; 12]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Key init failed: {}", e))?;
    let nonce = Nonce::from_slice(iv);
    
    cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// Zstandard 압축 (성능 우선 레벨 3 기본 사용)
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>, String> {
    zstd::encode_all(Cursor::new(data), level)
        .map_err(|e| format!("Zstd compression failed: {}", e))
}

// Zstandard 압축 해제
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, String> {
    zstd::decode_all(Cursor::new(data))
        .map_err(|e| format!("Zstd decompression failed: {}", e))
}
