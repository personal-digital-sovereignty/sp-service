use axum::{
    extract::ConnectInfo,
    http::{Request, StatusCode, Method},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::OnceLock;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NetworkIdentity {
    pub alias: String,
    pub jwt_secret: String,
    pub current_token: String,
}

pub static NETWORK_IDENTITY: OnceLock<NetworkIdentity> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn init_network_identity() -> NetworkIdentity {
    let id = uuid::Uuid::new_v4().to_string();
    let alias = format!("sovereign-{}", &id[0..4]);
    let jwt_secret = format!("{}-{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    
    let claims = Claims {
        sub: "sovereign_pairing".to_owned(),
        exp: (chrono::Utc::now() + chrono::Duration::try_days(30).unwrap()).timestamp() as usize,
    };
    
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_bytes())).unwrap();
    
    let identity = NetworkIdentity {
        alias,
        jwt_secret,
        current_token: token,
    };
    
    NETWORK_IDENTITY.set(identity.clone()).unwrap();
    identity
}

pub fn start_mdns_beacon(alias: &str, port: u16) {
    let mdns = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(e) => {
            tracing::error!("🚨 Falha Crítica ao alocar o Daemon mDNS da LAN: {}. O Modo Pareamento falhará, mas o Engine continuará rodando offline.", e);
            return;
        }
    };
    let service_type = "_http._tcp.local.";
    let instance_name = alias;
    let host_name = format!("{}.local.", alias);
    
    let mut properties = HashMap::new();
    properties.insert("app".to_string(), "sovereign-pair".to_string());
    
    match ServiceInfo::new(
        service_type,
        instance_name,
        &host_name,
        "", // mdns-sd uses this to infer automatically
        port,
        Some(properties)
    ) {
        Ok(my_service) => {
            if let Err(e) = mdns.register(my_service) {
                tracing::error!("Falha ao registrar serviço mDNS: {}", e);
            } else {
                tracing::info!("📡 Beacon mDNS ativado: {}.local na porta {}", alias, port);
            }
        }
        Err(e) => tracing::error!("Falha ao criar ServiceInfo mDNS: {}", e),
    }
}

pub async fn lan_auth_guard(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Permitir preflights CORS passarem sem payload JWT Auth
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let ip = addr.ip();
    
    // Bypass authentication for localhost (Trust Local Machine)
    if ip.is_loopback() {
        return Ok(next.run(req).await);
    }
    
    // For local network traffic, require JWT
    let auth_header = req.headers().get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
        
    let identity = NETWORK_IDENTITY.get().expect("Sovereign Identity not initialized on boot");
    
    if let Some(token) = auth_header {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        
        if decode::<Claims>(token, &DecodingKey::from_secret(identity.jwt_secret.as_bytes()), &validation).is_ok() {
            return Ok(next.run(req).await);
        }
    }
    
    tracing::warn!("🛡️ Zero-Trust Guard: Dispositivo invasor na LAN IP '{}' bloqueado por falta de token JWT válido.", ip);
    Err(StatusCode::UNAUTHORIZED)
}

pub async fn get_pairing_info_handler() -> impl axum::response::IntoResponse {
    if let Some(identity) = NETWORK_IDENTITY.get() {
        axum::Json(serde_json::json!({
            "alias": format!("{}.local", identity.alias),
            "token": identity.current_token
        }))
    } else {
        axum::Json(serde_json::json!({
            "error": "Identity not initialized"
        }))
    }
}
