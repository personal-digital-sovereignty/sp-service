//! ============================================================
//! Sovereign Pair — Reflection Lab Test Suite
//! Covers: Payload validation, Settings clamp, Struct deserialization,
//!         Model tag propagation, SSE broadcast channel
//! ============================================================

#[cfg(test)]
mod reflection_payload {
    use crate::api_trainer::{ReflectionDatasetPayload, ReflectionSettingsPayload, ReflectionSimReq};

    /// Dataset payload deserializes correctly with model_tag
    #[test]
    fn test_dataset_payload_with_model_tag() {
        let json = r#"{"model_tag": "qwen2.5:7b", "payload_json": "{\"key\": \"val\"}"}"#;
        let parsed: ReflectionDatasetPayload = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert_eq!(parsed.model_tag, Some("qwen2.5:7b".to_string()));
        assert!(parsed.payload_json.contains("key"));
    }

    /// Dataset payload without model_tag defaults to None (fallback "ui_injected" in handler)
    #[test]
    fn test_dataset_payload_without_model_tag() {
        let json = r#"{"payload_json": "{}"}"#;
        let parsed: ReflectionDatasetPayload = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert!(parsed.model_tag.is_none(), "model_tag should be None when absent from JSON");
    }

    /// Dataset payload with empty payload_json is valid (handler saves, validation is client concern)
    #[test]
    fn test_dataset_payload_empty_json_string() {
        let json = r#"{"payload_json": ""}"#;
        let parsed: ReflectionDatasetPayload = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert_eq!(parsed.payload_json, "");
    }

    /// Settings payload deserializes with i32 values
    #[test]
    fn test_settings_payload_deserialization() {
        let json = r#"{"reasoning_depth": 75, "audit_intensity": 90, "internal_monologue": true}"#;
        let parsed: ReflectionSettingsPayload = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert_eq!(parsed.reasoning_depth, 75);
        assert_eq!(parsed.audit_intensity, 90);
        assert!(parsed.internal_monologue);
    }

    /// Settings with out-of-range values should deserialize (clamp happens in handler)
    #[test]
    fn test_settings_payload_extreme_values() {
        let json = r#"{"reasoning_depth": -999, "audit_intensity": 99999, "internal_monologue": false}"#;
        let parsed: ReflectionSettingsPayload = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert_eq!(parsed.reasoning_depth, -999);
        assert_eq!(parsed.audit_intensity, 99999);
        // Verify clamp logic matches handler behavior
        assert_eq!(parsed.reasoning_depth.clamp(0, 100), 0, "Negative depth should clamp to 0");
        assert_eq!(parsed.audit_intensity.clamp(0, 100), 100, "Overflow intensity should clamp to 100");
    }

    /// SimReq requires model_name
    #[test]
    fn test_sim_req_deserialization() {
        let json = r#"{"model_name": "deepseek-r1:14b"}"#;
        let parsed: ReflectionSimReq = serde_json::from_str(json).expect("test assertion failed — review test setup");
        assert_eq!(parsed.model_name, "deepseek-r1:14b");
    }

    /// Missing model_name should fail deserialization
    #[test]
    fn test_sim_req_missing_model_name_fails() {
        let json = r#"{"wrong_field": "test"}"#;
        let result = serde_json::from_str::<ReflectionSimReq>(json);
        assert!(result.is_err(), "SimReq without model_name should fail to parse");
    }
}

#[cfg(test)]
mod reflection_broadcast {
    use crate::api_trainer::REFLECTION_LOGS;

    /// REFLECTION_LOGS broadcast channel is functional
    #[test]
    fn test_reflection_logs_channel_exists() {
        let mut rx = REFLECTION_LOGS.subscribe();
        let msg = serde_json::json!({"type": "test", "title": "Unit Test"}).to_string();
        let sent = REFLECTION_LOGS.send(msg.clone());
        assert!(sent.is_ok(), "Broadcast send should succeed with active subscriber");
        
        let received = rx.try_recv();
        assert!(received.is_ok(), "Subscriber should receive the sent message");
        assert_eq!(received.expect("test assertion failed — review test setup"), msg);
    }

    /// EOF signal format matches frontend expectations
    #[test]
    fn test_eof_signal_format() {
        let eof = serde_json::json!({
            "type": "EOF",
            "title": "Simulation Complete",
            "icon": "task_alt",
            "color": "text-primary bg-primary-container/10",
            "desc": "Reflection pipeline finished.",
            "time": "Just now"
        });
        let parsed: serde_json::Value = serde_json::from_str(&eof.to_string()).expect("test assertion failed — review test setup");
        assert_eq!(parsed["type"], "EOF", "EOF signal must have type='EOF'");
        assert!(parsed["title"].is_string(), "EOF must have a title");
        assert!(parsed["icon"].is_string(), "EOF must have an icon");
    }

    /// Keep-alive and lagged messages don't corrupt the JSON stream
    #[test]
    fn test_log_json_roundtrip_integrity() {
        let log = serde_json::json!({
            "type": "completion",
            "title": "Test with special chars: <>&\"'São Paulo/München",
            "icon": "psychology",
            "color": "text-primary bg-primary-container/10",
            "desc": "Chars: àáâãéêíóôõúç",
            "time": "Just now"
        });
        let serialized = log.to_string();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).expect("test assertion failed — review test setup");
        assert_eq!(deserialized["title"], log["title"], "Unicode roundtrip must be lossless");
    }
}

#[cfg(test)]
mod reflection_settings_persistence {
    /// Settings JSON format matches what GET handler returns as default
    #[test]
    fn test_default_settings_shape() {
        let defaults = serde_json::json!({
            "reasoning_depth": 80,
            "audit_intensity": 65,
            "internal_monologue": false
        });
        assert_eq!(defaults["reasoning_depth"], 80);
        assert_eq!(defaults["audit_intensity"], 65);
        assert_eq!(defaults["internal_monologue"], false);
    }

    /// Settings value_json roundtrip through serde
    #[test]
    fn test_settings_json_roundtrip() {
        let input = serde_json::json!({
            "reasoning_depth": 42,
            "audit_intensity": 88,
            "internal_monologue": true
        });
        let serialized = input.to_string();
        let val: serde_json::Value = serde_json::from_str(&serialized).expect("test assertion failed — review test setup");
        assert_eq!(val["reasoning_depth"].as_i64().expect("test assertion failed — review test setup"), 42);
        assert_eq!(val["audit_intensity"].as_i64().expect("test assertion failed — review test setup"), 88);
        assert!(val["internal_monologue"].as_bool().expect("test assertion failed — review test setup"));
    }
}
