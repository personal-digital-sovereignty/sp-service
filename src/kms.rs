use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use aes_gcm::aead::rand_core::RngCore;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use tracing::{info, warn};
use zeroize::Zeroize;

const KEK_ENV_VAR: &str = "SOVEREIGN_MASTER_KEK";

/// Obtém a chave mestra de criptografia (KEK).
/// Se não existir, gera uma chave aleatória de 256 bits (32 bytes),
/// salva no arquivo `.env` local e retorna.
fn get_or_generate_master_key() -> [u8; 32] {
    let key_b64 = env::var(KEK_ENV_VAR).unwrap_or_else(|_| {
        warn!("Módulo KMS: SOVEREIGN_MASTER_KEK não encontrada no ambiente. Gerando nova Chave de Segurança O.S...");
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let b64 = BASE64.encode(key_bytes);
        
        // Registra a chave gerada no .env (para persistência do sistema do usuário)
        if let Ok(mut file) = OpenOptions::new().append(true).open(".env") {
            let config_line = format!("\n# Gerado automaticamente pelo Módulo SecOps (KMS) - REQUIRED FOR DECRYPTION\n{}={}\n", KEK_ENV_VAR, b64);
            let _ = file.write_all(config_line.as_bytes());
            info!("Módulo KMS: Nova chave injetada no arquivo .env local.");
        } else {
            warn!("Módulo KMS: Falha ao escrever no .env local. A chave será perdida no próximo boot se não for salva.");
        }
        
        unsafe { env::set_var(KEK_ENV_VAR, &b64); }
        b64
    });

    let mut key_bytes = [0u8; 32];
    if let Ok(decoded) = BASE64.decode(&key_b64) {
        if decoded.len() == 32 {
            key_bytes.copy_from_slice(&decoded);
        } else {
            warn!("Módulo KMS: A chave SOVEREIGN_MASTER_KEK não possui 32 bytes válidos. Usando derivação/zero.");
        }
    }
    
    key_bytes
}

/// Encripta uma string em AES-GCM 256
pub fn encrypt_vault_secret(plaintext: &str) -> Option<String> {
    if plaintext.is_empty() { return None; }
    
    let key = get_or_generate_master_key();
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            // Empacota: [12 bytes nonce] + [ciphertext]
            let mut payload = nonce_bytes.to_vec();
            payload.extend(ciphertext);
            Some(BASE64.encode(payload))
        }
        Err(e) => {
            tracing::error!("KMS Encrypt Error: {:?}", e);
            None
        }
    }
}

/// Decifra uma string codificada AES-GCM 256. Zeroiza o plano após conversão.
pub fn decrypt_vault_secret(encrypted_b64: &str) -> Option<String> {
    if encrypted_b64.is_empty() { return None; }

    let payload = BASE64.decode(encrypted_b64).ok()?;
    if payload.len() < 12 { return None; }

    let key = get_or_generate_master_key();
    let cipher = Aes256Gcm::new(&key.into());

    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    match cipher.decrypt(nonce, ciphertext) {
        Ok(mut cleartext) => {
            let result = String::from_utf8(cleartext.clone()).ok();
            cleartext.zeroize(); // Memory Wipe Protection (Leak Prevention)
            result
        }
        Err(_) => None // Silencioso falhar se chave não casar
    }
}
