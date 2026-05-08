//! ============================================================
//! sp-service — Multimodal Tests
//! Tests for OCR, audio transcription, and node resolution
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::multimodal::{
        resolve_node_python, resolve_node_script,
        WhisperResult, OcrResult, MidiResult,
    };

    // ─────────────────────────────────────────────────────────
    // resolve_node_python Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_node_python_returns_valid_path() {
        let python = resolve_node_python();
        // Deve retornar "python", "python3" ou um path existente
        assert!(!python.is_empty());
        assert!(
            python.contains("python") || std::path::Path::new(&python).exists(),
            "resolve_node_python returned invalid path: {}",
            python
        );
    }

    #[test]
    fn test_resolve_node_python_not_empty() {
        let python = resolve_node_python();
        assert!(!python.trim().is_empty());
    }

    // ─────────────────────────────────────────────────────────
    // resolve_node_script Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_node_script_audio_transcriber() {
        let script = resolve_node_script("audio_transcriber");
        assert!(script.to_string_lossy().contains("audio_transcriber"));
        assert!(script.to_string_lossy().ends_with(".py"));
    }

    #[test]
    fn test_resolve_node_script_vision_ocr() {
        let script = resolve_node_script("vision_ocr");
        assert!(script.to_string_lossy().contains("vision_ocr"));
        assert!(script.to_string_lossy().ends_with(".py"));
    }

    #[test]
    fn test_resolve_node_script_midi_transcriber() {
        let script = resolve_node_script("midi_transcriber");
        assert!(script.to_string_lossy().contains("midi_transcriber"));
        assert!(script.to_string_lossy().ends_with(".py"));
    }

    #[test]
    fn test_resolve_node_script_fallback_path() {
        // Mesmo que o script não exista, o fallback deve gerar um path válido
        let script = resolve_node_script("nonexistent_node_xyz");
        assert!(script.to_string_lossy().contains("nonexistent_node_xyz"));
        assert!(script.to_string_lossy().ends_with(".py"));
    }

    // ─────────────────────────────────────────────────────────
    // Result Struct Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_whisper_result_default() {
        let result = WhisperResult {
            success: true,
            language: Some("pt".to_string()),
            language_probability: Some(0.95),
            text: Some("Olá mundo".to_string()),
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.language, Some("pt".to_string()));
        assert_eq!(result.text, Some("Olá mundo".to_string()));
    }

    #[test]
    fn test_whisper_result_error() {
        let result = WhisperResult {
            success: false,
            language: None,
            language_probability: None,
            text: None,
            error: Some("Model not found".to_string()),
        };
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_ocr_result_default() {
        let result = OcrResult {
            success: true,
            text: Some("Texto extraído".to_string()),
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.text, Some("Texto extraído".to_string()));
    }

    #[test]
    fn test_ocr_result_error() {
        let result = OcrResult {
            success: false,
            text: None,
            error: Some("Image not readable".to_string()),
        };
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_midi_result_default() {
        let result = MidiResult {
            success: true,
            output_dir: Some("/tmp/midi".to_string()),
            message: Some("Conversion complete".to_string()),
            error: None,
        };
        assert!(result.success);
        assert_eq!(result.output_dir, Some("/tmp/midi".to_string()));
    }

    #[test]
    fn test_midi_result_error() {
        let result = MidiResult {
            success: false,
            output_dir: None,
            message: None,
            error: Some("Failed to detect notes".to_string()),
        };
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    // ─────────────────────────────────────────────────────────
    // Serialization Tests
    // ─────────────────────────────────────────────────────────

    #[test]
    fn test_whisper_result_serialize() {
        let result = WhisperResult {
            success: true,
            language: Some("en".to_string()),
            language_probability: Some(0.88),
            text: Some("Hello world".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("language"));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn test_whisper_result_deserialize() {
        let json = r#"{"success":true,"language":"fr","language_probability":0.92,"text":"Bonjour","error":null}"#;
        let result: WhisperResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.language, Some("fr".to_string()));
        assert_eq!(result.text, Some("Bonjour".to_string()));
    }

    #[test]
    fn test_ocr_result_serialize() {
        let result = OcrResult {
            success: true,
            text: Some("OCR text".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("OCR text"));
    }

    #[test]
    fn test_midi_result_serialize() {
        let result = MidiResult {
            success: true,
            output_dir: Some("/tmp/output".to_string()),
            message: Some("Done".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("/tmp/output"));
        assert!(json.contains("Done"));
    }
}
