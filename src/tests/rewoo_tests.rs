//! ============================================================
//! sp-service — ReWOO Tests
//! Tests for ReWOO plan struct and dispatcher logic
//! ============================================================

#[cfg(test)]
mod tests {
    use crate::rewoo::{RewooPlan, RewooStep};

    #[tokio::test]
    async fn test_dispatch_planner_with_vault_query() {
        let plan = crate::rewoo::HybridRouter::dispatch_planner("Search @vault for data").await;
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].worker, "VaultSearch");
        assert_eq!(plan.steps[0].id, "E1");
    }

    #[tokio::test]
    async fn test_dispatch_planner_without_vault() {
        let plan = crate::rewoo::HybridRouter::dispatch_planner("What is the weather?").await;
        assert_eq!(plan.steps.len(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_planner_case_insensitive() {
        let plan = crate::rewoo::HybridRouter::dispatch_planner("Find @VAULT records").await;
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_rewoo_plan_serialization() {
        let plan = RewooPlan {
            steps: vec![
                RewooStep {
                    id: "E1".to_string(),
                    worker: "TestWorker".to_string(),
                    args: vec!["arg1".to_string()],
                }
            ],
        };
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("E1"));
        assert!(json.contains("TestWorker"));
    }

    #[test]
    fn test_rewoo_step_fields() {
        let step = RewooStep {
            id: "E2".to_string(),
            worker: "Worker".to_string(),
            args: vec!["a".to_string(), "b".to_string()],
        };
        assert_eq!(step.id, "E2");
        assert_eq!(step.args.len(), 2);
    }

    #[test]
    fn test_rewoo_plan_empty() {
        let plan = RewooPlan { steps: vec![] };
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_rewoo_plan_debug() {
        let plan = RewooPlan {
            steps: vec![
                RewooStep {
                    id: "D1".to_string(),
                    worker: "DebugWorker".to_string(),
                    args: vec![],
                }
            ],
        };
        let debug_str = format!("{:?}", plan);
        assert!(debug_str.contains("RewooPlan"));
    }
}
