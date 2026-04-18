use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperResult {
    pub success: bool,
    pub language: Option<String>,
    pub language_probability: Option<f64>,
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrResult {
    pub success: bool,
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MidiResult {
    pub success: bool,
    pub output_dir: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// WIN-05/MacOS: Resolve python executável + path do node de forma robusta.
/// Usa o venv hermético da Sandbox se disponível, senão fallback ao python do sistema.
fn resolve_node_python() -> String {
    let hermetic = crate::sandbox::get_hermetic_python_bin();
    if hermetic.exists() {
        hermetic.to_string_lossy().to_string()
    } else if cfg!(target_os = "windows") {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}

/// Resolve o caminho absoluto para um script `../nodes/<name>.py` de forma dinâmica.
/// Funciona em dev (cargo workspace), MacOS App Bundle e Windows.
fn resolve_node_script(name: &str) -> std::path::PathBuf {
    let script_name = format!("{}.py", name);

    // Tentativa 1: junto ao python_workers (workspace raiz Cargo)
    let workers = crate::api_trainer::resolve_python_workers_dir();
    {
        let candidate = workers.parent().unwrap_or(&workers).join("nodes").join(&script_name);
        if candidate.exists() { return candidate; }
    }
    // Tentativa 2: relativo ao executável (MacOS Bundle / Windows install)
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent().unwrap_or(std::path::Path::new(".")).join("nodes").join(&script_name);
        if candidate.exists() { return candidate; }
        // MacOS Bundle: Contents/Resources/nodes
        let candidate2 = exe.parent().unwrap_or(std::path::Path::new("."))
            .parent().unwrap_or(std::path::Path::new("."))
            .join("Resources").join("nodes").join(&script_name);
        if candidate2.exists() { return candidate2; }
    }
    // Fallback: caminho relativo original (funciona em dev Linux)
    std::path::PathBuf::from("../nodes").join(&script_name)
}

pub async fn extract_text_from_audio(file_path: &str) -> Result<WhisperResult, String> {
    info!("Iniciando node Python Whisper para: {}", file_path);
    let python = resolve_node_python();
    let script = resolve_node_script("audio_transcriber");
    let output = Command::new(&python)
        .arg(&script)
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Falha ao invocar processo Node: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        error!("Erro no Node Whisper Python: {}", stderr);
    }

    match serde_json::from_str::<WhisperResult>(&stdout) {
        Ok(mut res) => {
            if !output.status.success() && res.error.is_none() {
                res.error = Some(stderr.to_string());
            }
            Ok(res)
        }
        Err(e) => {
            Err(format!("Falha ao fazer parse do JSON do Node Whisper: {} | Saida Bruta: {}", e, stdout))
        }
    }
}

pub async fn extract_text_from_image(file_path: &str) -> Result<OcrResult, String> {
    info!("Iniciando node Python PaddleOCR para: {}", file_path);
    let python = resolve_node_python();
    let script = resolve_node_script("vision_ocr");
    let output = Command::new(&python)
        .arg(&script)
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Falha ao invocar processo Node: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        error!("Erro no Node PaddleOCR Python: {}", stderr);
    }

    match serde_json::from_str::<OcrResult>(&stdout) {
        Ok(mut res) => {
            if !output.status.success() && res.error.is_none() {
                res.error = Some(stderr.to_string());
            }
            Ok(res)
        }
        Err(e) => {
            Err(format!("Falha ao fazer parse do JSON do Node PaddleOCR: {} | Saida Bruta: {}", e, stdout))
        }
    }
}

pub async fn extract_midi_from_audio(audio_path: &str, output_dir: &str) -> Result<MidiResult, String> {
    info!("Iniciando node Python Basic Pitch para: {}", audio_path);
    let python = resolve_node_python();
    let script = resolve_node_script("midi_transcriber");
    let output = Command::new(&python)
        .arg(&script)
        .arg(audio_path)
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Falha ao invocar processo Node: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        error!("Erro no Node Basic Pitch Python: {}", stderr);
    }

    match serde_json::from_str::<MidiResult>(&stdout) {
        Ok(mut res) => {
            if !output.status.success() && res.error.is_none() {
                res.error = Some(stderr.to_string());
            }
            Ok(res)
        }
        Err(e) => {
            Err(format!("Falha ao fazer parse do JSON do Node Basic Pitch: {} | Saida Bruta: {}", e, stdout))
        }
    }
}
