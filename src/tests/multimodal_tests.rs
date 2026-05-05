#[cfg(test)]
mod tests {
    use crate::multimodal::{WhisperResult, OcrResult, MidiResult};

    #[test]
    fn test_whisper_result_deserialize() {
        let json = r#"{
            "success": true,
            "language": "pt",
            "language_probability": 0.99,
            "text": "Hello world",
            "error": null
        }"#;
        
        let result: WhisperResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.language.unwrap(), "pt");
        assert_eq!(result.text.unwrap(), "Hello world");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_ocr_result_deserialize() {
        let json = r#"{
            "success": false,
            "text": null,
            "error": "Failed to read image"
        }"#;
        
        let result: OcrResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert!(result.text.is_none());
        assert_eq!(result.error.unwrap(), "Failed to read image");
    }

    #[test]
    fn test_midi_result_deserialize() {
        let json = r#"{
            "success": true,
            "output_dir": "/tmp/midi",
            "message": "Conversion completed",
            "error": null
        }"#;
        
        let result: MidiResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.output_dir.unwrap(), "/tmp/midi");
        assert_eq!(result.message.unwrap(), "Conversion completed");
    }
}
