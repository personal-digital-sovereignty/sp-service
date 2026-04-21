// ============================================================
// Sovereign Pair — Resilience Shield: API Health Gate
//
// Runs health_check_apis.py --json at startup and periodically,
// persisting results in api_health_log and exposing them via SSE
// for frontend degradation badges.
// ============================================================

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Represents a single API health check result (matches Python output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHealthEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub api_type: String,
    pub status: String,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub latency_ms: i64,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub records: Option<i64>,
    #[serde(default)]
    pub latest_date: Option<String>,
    #[serde(default)]
    pub latest_value: Option<String>,
}

/// Aggregated health status for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHealthSummary {
    pub total_apis: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub critical_failures: Vec<String>,
    pub last_checked: String,
    pub entries: Vec<ApiHealthEntry>,
}

impl Default for ApiHealthSummary {
    fn default() -> Self {
        Self {
            total_apis: 0,
            healthy: 0,
            degraded: 0,
            critical_failures: vec![],
            last_checked: "Never".to_string(),
            entries: vec![],
        }
    }
}

/// Shared state for the latest health check results
pub type HealthState = Arc<RwLock<ApiHealthSummary>>;

pub fn new_health_state() -> HealthState {
    Arc::new(RwLock::new(ApiHealthSummary::default()))
}

/// Execute health_check_apis.py --json and parse the results
pub async fn run_health_check() -> Result<Vec<ApiHealthEntry>, String> {
    let python_bin = crate::sandbox::get_hermetic_python_bin();
    let workers_dir = crate::api_trainer::resolve_python_workers_dir();
    let script_path = workers_dir.join("health_check_apis.py");

    if !script_path.exists() {
        return Err(format!("Health check script not found: {:?}", script_path));
    }

    info!("🛡️ [Resilience Shield] Running API health check: {:?}", script_path);

    let output = tokio::process::Command::new(&python_bin)
        .arg(&script_path)
        .arg("--json")
        .output()
        .await
        .map_err(|e| format!("Failed to spawn health check: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Health check may return exit code 1 for critical failures but still outputs valid JSON
        if output.stdout.is_empty() {
            return Err(format!("Health check failed with no output: {}", stderr));
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<Vec<ApiHealthEntry>>(&stdout)
        .map_err(|e| format!("Failed to parse health check JSON: {} — raw: {}", e, &stdout[..stdout.len().min(200)]))
}

/// Persist health check results to SQLite and enforce 7-day retention
pub async fn persist_health_results(db: &SqlitePool, entries: &[ApiHealthEntry]) {
    for entry in entries {
        let result = sqlx::query(
            "INSERT INTO api_health_log (api_name, api_type, status, is_critical, latency_ms, error_message) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&entry.name)
        .bind(&entry.api_type)
        .bind(&entry.status)
        .bind(entry.critical)
        .bind(entry.latency_ms)
        .bind(&entry.error)
        .execute(db)
        .await;

        if let Err(e) = result {
            error!("🛡️ [Resilience Shield] Failed to persist health entry '{}': {}", entry.name, e);
        }
    }

    // GAP-RS-05: Enforce 7-day retention to prevent unbounded growth
    let _ = sqlx::query("DELETE FROM api_health_log WHERE checked_at < datetime('now', '-7 days')")
        .execute(db)
        .await;
}

/// Build a summary from health check entries
pub fn build_summary(entries: Vec<ApiHealthEntry>) -> ApiHealthSummary {
    let healthy = entries.iter().filter(|e| e.status == "HEALTHY" || e.status == "SKIP").count();
    let degraded = entries.iter().filter(|e| e.status != "HEALTHY" && e.status != "SKIP").count();
    let critical_failures: Vec<String> = entries.iter()
        .filter(|e| e.critical && e.status != "HEALTHY")
        .map(|e| e.name.clone())
        .collect();

    if !critical_failures.is_empty() {
        warn!("🚨 [Resilience Shield] CRITICAL API FAILURES: {:?}", critical_failures);
    } else if degraded > 0 {
        warn!("⚠️ [Resilience Shield] {}/{} APIs degraded (non-critical)", degraded, entries.len());
    } else {
        info!("✅ [Resilience Shield] All {} APIs healthy", entries.len());
    }

    ApiHealthSummary {
        total_apis: entries.len(),
        healthy,
        degraded,
        critical_failures,
        last_checked: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        entries,
    }
}

/// Startup health gate: runs on server boot and updates shared state
pub async fn startup_health_gate(db: SqlitePool, health_state: HealthState) {
    match run_health_check().await {
        Ok(entries) => {
            persist_health_results(&db, &entries).await;
            let summary = build_summary(entries);
            *health_state.write().await = summary;
        }
        Err(e) => {
            error!("🛡️ [Resilience Shield] Startup health check failed: {}", e);
            // Don't block startup — mark as unknown
            let mut state = health_state.write().await;
            state.last_checked = format!("FAILED: {}", e);
        }
    }
}

/// Periodic watchdog: re-checks every 4 hours with Auto-Heal Anti-Panic architecture (TD-RS-01)
pub async fn spawn_periodic_watchdog(db: SqlitePool, health_state: HealthState) {
    // 🛡️ Supervisor Thread: Imune aos pânicos internos
    tokio::spawn(async move {
        loop {
            let db_clone = db.clone();
            let health_clone = health_state.clone();
            
            // Sub-thread Isolada (se crashar por unwrap ou parser quebrado, só ela cai)
            let handle = tokio::spawn(async move {
                let interval = std::time::Duration::from_secs(4 * 60 * 60);
                loop {
                    tokio::time::sleep(interval).await;
                    info!("🛡️ [Resilience Shield] Periodic health check triggered (4h interval)");
                    match run_health_check().await {
                        Ok(entries) => {
                            persist_health_results(&db_clone, &entries).await;
                            let summary = build_summary(entries);
                            *health_clone.write().await = summary;
                        }
                        Err(e) => {
                            error!("🛡️ [Resilience Shield] Periodic health check failed: {}", e);
                            // GAP-RS-03: Update timestamp even on failure so frontend never shows stale data
                            let mut state = health_clone.write().await;
                            state.last_checked = format!("{} (check failed: {})",
                                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                &e[..e.len().min(60)]
                            );
                        }
                    }
                }
            });

            // Se o join falhar, a thread filha entrou em Panic/Abort. Acionamos o Rescue!
            match handle.await {
                Ok(_) => {
                    // Loop infinito saiu limpo — quebramos a main thread gracefully
                    break;
                }
                Err(e) => {
                    error!("🚨 [CRITICAL GAP EVADED] Watchdog Health Thread sofreu PANIC Interno: {}. Respawning em 60s...", e);
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            }
        }
    });
}

// ── Axum Handler: GET /v1/analytics/api_health ──

pub async fn api_health_handler(
    axum::extract::State(state): axum::extract::State<Arc<crate::AppState>>,
) -> axum::response::Json<serde_json::Value> {
    let health = state.health.read().await;
    axum::response::Json(serde_json::json!({
        "total_apis": health.total_apis,
        "healthy": health.healthy,
        "degraded": health.degraded,
        "critical_failures": health.critical_failures,
        "last_checked": health.last_checked,
        "entries": health.entries,
    }))
}
