use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::task::JoinHandle;

// The JSON Structure the Planner LLM will return
#[derive(Debug, Serialize, Deserialize)]
pub struct RewooPlan {
    pub steps: Vec<RewooStep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RewooStep {
    pub id: String,
    pub worker: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct RewooResult {
    pub step_id: String,
    pub output: String,
}

// Intercepts the query and initiates the ReWOO DAG Execution
pub async fn execute_rewoo_plan(user_query: &str, vault_path: &std::path::PathBuf) -> String {
    info!("🧠 [ReWOO Orchestrator] Intercepting Query for Planning: {}", user_query);
    
    // Todo: Call the Local LLM / Oracle LLM to generate the JSON DAG Plan.
    // For now we simulate a manual plan to avoid breaking the compilation.
    let plan = RewooPlan {
        steps: vec![
            RewooStep {
                id: "E1".to_string(),
                worker: "VaultSearch".to_string(),
                args: vec![user_query.to_string()],
            }
        ]
    };

    let mut handles: Vec<JoinHandle<RewooResult>> = vec![];

    // Spawn Parallel Executors
    for step in plan.steps {
        let worker_cmd = step.worker.clone();
        let args = step.args.clone();
        let step_id = step.id.clone();
        
        let v_path = vault_path.clone();

        let handle = tokio::spawn(async move {
            let output = match worker_cmd.as_str() {
                "VaultSearch" => {
                    info!("🔍 [ReWOO Worker {}] Running VaultSearch Async...", step_id);
                    // Abusing existing Rag logic for now
                    crate::rag::parse_vault_documents(&v_path)
                },
                "Telemetry" => {
                    "System IO at 100%".to_string()
                },
                _ => {
                    warn!("⚠️ [ReWOO] Unknown Worker: {}", worker_cmd);
                    "No Result".to_string()
                }
            };

            RewooResult { step_id, output }
        });
        handles.push(handle);
    }

    // Await all parallel workers to collect observations
    let mut observations = String::new();
    for handle in handles {
        if let Ok(res) = handle.await {
            observations.push_str(&format!("\n[Observation {}]: {}", res.step_id, res.output));
        }
    }

    info!("✅ [ReWOO Orchestrator] All parallel tasks finished. Solving...");
    
    // We append the compiled observations back to be injected into the LLM
    format!("ReWOO Accumulated Observations:\n{}", observations)
}
