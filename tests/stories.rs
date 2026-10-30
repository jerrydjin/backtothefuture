use muse::negotiate::ScriptedTalk;
use muse::network::run_story_town;
use muse::story::{
    resolve_competing, PriorityLedger, PriorityReason, RiderClaim, StoryKind, StoryRun,
};

#[test]
fn repeated_weak_requests_stop_winning_even_with_new_ids_and_cars() {
    let mut ledger = PriorityLedger::default();
    let first = RiderClaim {
        rider_id: 5,
        car_id: 177,
        sequence: 1,
        at_s: 200.0,
        reason: PriorityReason::Meeting,
    };
    assert!(ledger.resolve(&[first.clone()])[0].granted);
    assert!(
        !ledger.resolve(&[first.clone()])[0].granted,
        "resending the same request cannot grant another slot"
    );
    for n in 2..20 {
        let claim = RiderClaim {
            sequence: n,
            car_id: 177 + n,
            at_s: 200.0 + f64::from(n),
            ..first.clone()
        };
        let denied = &ledger.resolve(&[claim])[0];
        assert!(!denied.granted, "weak request {n} won");
        assert!(denied.explanation.contains("repeated weak"));
    }
    let strong = RiderClaim {
        sequence: 20,
        at_s: 230.0,
        reason: PriorityReason::Hospital,
        ..first.clone()
    };
    assert!(
        ledger.resolve(&[strong])[0].granted,
        "real urgency still outranks weak history"
    );
    let quiet = RiderClaim {
        sequence: 21,
        at_s: 400.0,
        ..first
    };
    assert!(
        ledger.resolve(&[quiet])[0].granted,
        "quiet time restores ordinary fairness"
    );
}

#[test]
fn denial_does_not_extend_the_weak_physical_priority_slot() {
    let run = run_story_town(7, StoryKind::Abuse, Box::new(ScriptedTalk));
    let story = run.story.as_ref().unwrap();
    assert_eq!(story.decisions.iter().filter(|d| d.granted).count(), 1);
    assert_eq!(story.priority_until_s, Some(230.0));
    assert!(story.decisions[1..]
        .iter()
        .all(|d| !d.granted && d.explanation.starts_with("No:")));
    assert!(run.exchanges.iter().all(|ex| ex.t < 230.0));
    assert!(
        run.exchanges.len() < 2,
        "denied requests cannot buy the second junction"
    );
    assert_eq!(run.stats.overlaps + run.stats.crossing_conflicts, 0);
}

#[test]
fn hospital_wins_simultaneous_requests_in_either_input_order() {
    let story = StoryRun::new(StoryKind::Competing);
    for claims in [
        story.claims.clone(),
        story.claims.iter().rev().cloned().collect(),
    ] {
        assert_eq!(claims[0].at_s, claims[1].at_s);
        let decisions = resolve_competing(&claims);
        let wins: Vec<_> = decisions.iter().filter(|d| d.granted).collect();
        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].claim.reason, PriorityReason::Hospital);
        assert_eq!(
            wins[0].explanation,
            "Hospital takes priority over a meeting."
        );
        assert!(!wins[0].explanation.contains('\n'));
    }
}

#[test]
fn hospital_decision_is_carried_into_the_physical_yields() {
    let run = run_story_town(7, StoryKind::Competing, Box::new(ScriptedTalk));
    let story = run.story.as_ref().unwrap();
    let hospital = story.winner().unwrap();
    assert_eq!(hospital.car_id, 177);
    assert!(
        !story
            .decisions
            .iter()
            .find(|d| d.claim.car_id == 200)
            .unwrap()
            .granted
    );
    assert!(story
        .claims
        .iter()
        .all(|claim| run.frames[200].cars.iter().any(|c| c.0 == claim.car_id)));
    assert_eq!(run.exchanges.len(), 2);
    assert_eq!(
        run.exchanges[0].yields, 200,
        "the meeting car yields at the first contested junction"
    );
    for ex in &run.exchanges {
        assert_eq!(ex.priority_id, Some(hospital.car_id));
        assert!(ex
            .forecast
            .cars
            .iter()
            .all(|c| c.pref.contains(story.explanation())));
    }
    assert_eq!(run.priority.as_ref().unwrap().time_home_s, Some(126.5));
    assert_eq!(run.stats.overlaps + run.stats.crossing_conflicts, 0);
}
