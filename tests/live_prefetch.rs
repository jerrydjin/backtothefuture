use malleable::rush_hour::{self, Decision, Mock, Policy, PolicyRequest};
use std::sync::Mutex;

#[derive(Default)]
struct Probe {
    prepared: Mutex<Vec<PolicyRequest>>,
    assessed: Mutex<Vec<PolicyRequest>>,
}
impl Policy for Probe {
    fn name(&self) -> &str {
        "mock"
    }
    fn prepare(&self, requests: &[PolicyRequest]) {
        *self.prepared.lock().unwrap() = requests.to_vec();
    }
    fn decide(&self, r: &PolicyRequest) -> Decision {
        self.assessed.lock().unwrap().push(r.clone());
        Mock.decide(r)
    }
}

#[test]
fn prefetch_contains_only_independent_claims_and_preserves_causal_history() {
    let probe = Probe::default();
    let run = rush_hour::simulate(1, &probe).unwrap();
    let prepared = probe.prepared.lock().unwrap();
    let assessed = probe.assessed.lock().unwrap();
    assert_eq!(prepared.len(), 216);
    let mut riders = std::collections::BTreeSet::new();
    for r in prepared.iter() {
        assert!(riders.insert(r.rider));
        assert!(r.history.is_empty());
        let actual = assessed.iter().find(|a| a.car == r.car).unwrap();
        assert_eq!(
            serde_json::to_vec(r).unwrap(),
            serde_json::to_vec(actual).unwrap()
        );
    }
    assert!(assessed.iter().any(|r| !r.history.is_empty()));
    assert!(assessed
        .iter()
        .all(|r| r.history.iter().all(|h| h.completed_at_s <= r.at_s)));
    assert_eq!(run.decisions.len(), 240);
}

#[test]
fn score_counts_each_fallback_cause_without_changing_mock_wire_bytes() {
    let live = rush_hour::Live::with_client_budget(None, std::time::Duration::ZERO);
    let result = rush_hour::run(1, &live).unwrap();
    assert_eq!(result.score.fallback_decisions, 240);
    assert_eq!(result.score.fallback_causes.get("no_key"), Some(&240));
    let result = rush_hour::run(1, &Mock).unwrap();
    assert!(!serde_json::to_string(&result.score)
        .unwrap()
        .contains("fallback_causes"));
}
