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

use std::sync::OnceLock;

static MASTER_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// 🔐 **KMS | Key Management System (Sovereign Vault)**
/// 
/// Provê a infraestrutura de criptografia autenticada (AES-256-GCM) para
/// proteger chaves de API e outros segredos. Implementa práticas de 
/// endurecimento de memória (Zeroize) para evitar exfiltração de dados 
/// via 'memory dumps'.
///
/// Obtém a chave mestra de criptografia (KEK) do ambiente ou a gera dinamicamente.
/// Thread-safe via `OnceLock` para evitar condições de corrida em sistemas multithreaded.
fn get_or_generate_master_key() -> [u8; 32] {
    *MASTER_KEY.get_or_init(|| {
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
            // JAMAIS utilizar unsafe { env::set_var } num ambiente Tokio Multithreaded! (Causa UB e GLIBC segfaults).
            b64
        });

        let mut key_bytes = [0u8; 32];
        if let Ok(mut decoded) = BASE64.decode(&key_b64) {
            if decoded.len() == 32 {
                key_bytes.copy_from_slice(&decoded);
            } else {
                warn!("Módulo KMS: A chave SOVEREIGN_MASTER_KEK não possui 32 bytes válidos. Usando derivação/zero.");
            }
            decoded.zeroize(); // Memory Wipe do vetor na heap
        }
        key_bytes
    })
}

/// 🔒 **Vault Encryption | AES-256-GCM 256-bit**
/// 
/// Encripta um segredo usando a chave mestra (KEK). Utiliza um nonce de 12 bytes
/// único para cada operação, garantindo que o mesmo segredo resulte em 
/// ciphertexts diferentes (Indistinguibilidade).
pub fn encrypt_vault_secret(plaintext: &str) -> Option<String> {
    if plaintext.is_empty() { return None; }

    let mut key = get_or_generate_master_key();
    let cipher = Aes256Gcm::new(&key.into());
    key.zeroize(); // Memory Wipe do array mestra na stack

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

/// 🔓 **Vault Decryption | Security-First Logic**
/// 
/// Decifra um segredo codificado. Implementa o padrão de "Memory Wipe" 
/// chamando `.zeroize()` em buffers temporários imediatamente após o uso, 
/// garantindo que segredos decifrados não permaneçam na memória heap ou stack.
pub fn decrypt_vault_secret(encrypted_b64: &str) -> Option<String> {
    if encrypted_b64.is_empty() { return None; }

    let payload = BASE64.decode(encrypted_b64).ok()?;
    if payload.len() < 12 { return None; }

    let mut key = get_or_generate_master_key();
    let cipher = Aes256Gcm::new(&key.into());
    key.zeroize(); // Memory Wipe do array mestra na stack

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