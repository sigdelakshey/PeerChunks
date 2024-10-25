use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use cipher::InvalidLength;
use rand::RngCore;
use hex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Hex decoding error: {0}")]
    HexDecodeError(#[from] hex::FromHexError),

    #[error("Invalid key length: {0}")]
    InvalidKeyLength(String),

    #[error("AES-GCM operation failed")]
    AesGcmError,
}

impl From<aes_gcm::Error> for EncryptionError {
    fn from(_error: aes_gcm::Error) -> Self {
        EncryptionError::AesGcmError
    }
}

impl From<InvalidLength> for EncryptionError {
    fn from(_error: InvalidLength) -> Self {
        EncryptionError::InvalidKeyLength("Invalid key length provided.".into())
    }
}

pub fn encrypt(data: &[u8], key: &str) -> Result<(String, String), EncryptionError> {
    let key_bytes = hex::decode(key)?;
    if key_bytes.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength(format!(
            "Expected 32 bytes, got {} bytes",
            key_bytes.len()
        )));
    }

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, data)?;

    Ok((
        hex::encode(nonce_bytes),
        hex::encode(ciphertext),
    ))
}

pub fn decrypt(nonce_hex: &str, ciphertext_hex: &str, key: &str) -> Result<Vec<u8>, EncryptionError> {
    let key_bytes = hex::decode(key)?;
    let nonce_bytes = hex::decode(nonce_hex)?;
    let ciphertext = hex::decode(ciphertext_hex)?;

    if key_bytes.len() != 32 || nonce_bytes.len() != 12 {
        return Err(EncryptionError::InvalidKeyLength(format!(
            "Invalid key or nonce length: key {} bytes, nonce {} bytes",
            key_bytes.len(),
            nonce_bytes.len()
        )));
    }

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let decrypted_data = cipher.decrypt(nonce, ciphertext.as_ref())?;

    Ok(decrypted_data)
}
