use malleable::negotiate::ScriptedTalk;
use malleable::network::run_story_town;
use malleable::story::{
    resolve_competing, PriorityLedger, PriorityReason, RiderClaim, StoryKind, StoryRun,
};

#[test]
fn sustained_distress_detects_an_emergency_but_noise_and_missing_samples_do_not() {
    use malleable::story::{detect_emergencies, SensorObservation};
    let sample = |t, distress| SensorObservation {
        car_id: 177,
        at_s: t,
        passenger_distress: distress,
    };
    assert!(
        detect_emergencies(&[sample(0.0, true), sample(1.0, false), sample(2.0, true)]).is_empty()
    );
    assert!(
        detect_emergencies(&[sample(0.0, true), sample(0.0, true), sample(2.0, true)]).is_empty()
    );
    assert!(
        detect_emergencies(&[sample(0.0, true), sample(5.0, true), sample(6.0, true)]).is_empty()
    );
    let detected = detect_emergencies(&[
        sample(0.0, true),
        sample(1.0, true),
        sample(2.0, true),
        sample(3.0, true),
    ]);
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].at_s, 2.0);
}

#[test]
fn detected_emergency_clears_the_route_without_a_rider_request() {
    let run = run_story_town(7, StoryKind::Emergency, Box::new(ScriptedTalk));
    let story = run.story.as_ref().unwrap();
    assert!(story.claims.is_empty(), "no human request was submitted");
    assert_eq!(story.detections.len(), 1);
    assert_eq!(story.detections[0].at_s, 200.0);
    assert_eq!(
        story.winner().unwrap().reason,
        PriorityReason::DetectedEmergency
    );
    assert_eq!(run.exchanges.len(), 2);
    assert_eq!(run.priority.as_ref().unwrap().time_home_s, Some(126.5));
    assert_eq!(run.stats.overlaps + run.stats.crossing_conflicts, 0);
}

#[test]
fn a_chain_contains_three_real_cars_and_relays_the_same_detected_emergency() {
    let run = run_story_town(7, StoryKind::Chain, Box::new(ScriptedTalk));
    assert!(run.story.as_ref().unwrap().claims.is_empty());
    assert_eq!(run.exchanges.len(), 2);
    for ex in &run.exchanges {
        let ids: std::collections::BTreeSet<_> = ex.lines.iter().map(|l| l.id).collect();
        assert!(ids.len() >= 3, "pairwise dialogue alone is not a chain");
        assert!(ids.contains(&177));
        assert_ne!(
            ex.merges, 177,
            "the requested car cannot jump past its relays"
        );
        assert!(ids.iter().all(|id| run.frames[ex.t.ceil() as usize]
            .cars
            .iter()
            .any(|c| c.0 == *id)));
        assert!(ex.forecast.cause.as_ref().unwrap().contains("distress"));
        assert!(ex
            .lines
            .iter()
            .all(|l| l.say.split_whitespace().count() <= 8));
    }
    assert_eq!(run.priority.as_ref().unwrap().time_home_s, Some(126.5));
    assert_eq!(run.stats.overlaps + run.stats.crossing_conflicts, 0);
}

#[test]
fn every_chain_participant_must_acknowledge_and_only_a_mouth_car_can_yield() {
    use malleable::negotiate::choice_from_reply;
    let run = run_story_town(7, StoryKind::Chain, Box::new(ScriptedTalk));
    let ex = &run.exchanges[0];
    let reply = |lines: &[malleable::negotiate::MergeLine], yields| {
        serde_json::json!({"lines":lines,"yields":yields}).to_string()
    };
    assert_eq!(
        choice_from_reply(&ex.forecast, Ok(&reply(&ex.lines, ex.yields))).source,
        "model"
    );
    let missing = choice_from_reply(&ex.forecast, Ok(&reply(&ex.lines[..2], ex.yields)));
    assert_eq!(missing.source, "fallback");
    assert_eq!(missing.reason, "parse");
    assert_eq!(missing.lines.len(), 3);
    let jump = choice_from_reply(&ex.forecast, Ok(&reply(&ex.lines, 177)));
    assert_eq!(jump.source, "fallback");
    assert_eq!(jump.merges, ex.merges);
}

#[test]
fn an_accepted_counter_offer_waits_for_the_actual_crossing_before_priority() {
    let run = run_story_town(7, StoryKind::CounterOffer, Box::new(ScriptedTalk));
    let ex = &run.exchanges[0];
    let offer = ex.counter_offer.as_ref().expect("a real counter-offer");
    for pair in run.exchanges.windows(2) {
        assert!(
            pair[1].t - 5.0 >= pair[0].until + pair[0].fade_s,
            "successive dialogue remains readable"
        );
    }
    assert_eq!(offer.by, ex.yields);
    assert_eq!(offer.accepted_by, ex.merges);
    assert!(
        ex.activation_t > ex.t + offer.wait_s,
        "a timer alone cannot clear an occupied crossing"
    );
    let car = |t: usize, id| run.frames[t].cars.iter().find(|c| c.0 == id).unwrap();
    let head = car(ex.t.ceil() as usize, ex.merges);
    let route = &run.routes[head.1 as usize];
    let k = route.iter().position(|id| *id == ex.stream_link).unwrap();
    let line: f64 = route[..=k].iter().map(|&id| run.links[id].len).sum();
    for t in ex.t.ceil() as usize..=ex.activation_t.floor() as usize {
        assert!(
            car(t, ex.merges).2 < line,
            "priority stream entered before the accepted crossing cleared"
        );
    }
    assert!(car(ex.activation_t.ceil() as usize + 4, ex.merges).2 > line);
    assert!(
        car(ex.activation_t.floor() as usize, offer.by).2
            > car(ex.t.ceil() as usize, offer.by).2 + 5.0
    );
    assert_eq!(run.stats.overlaps + run.stats.crossing_conflicts, 0);
    assert!(run.priority.as_ref().unwrap().time_home_s.unwrap() < 141.0);
}

#[test]
fn a_counter_offer_needs_explicit_acceptance_and_a_bounded_delay() {
    use malleable::negotiate::choice_from_reply;
    let run = run_story_town(7, StoryKind::CounterOffer, Box::new(ScriptedTalk));
    let ex = &run.exchanges[0];
    let mut body =
        serde_json::json!({"lines":ex.lines,"yields":ex.yields,"counter_offer":ex.counter_offer});
    assert_eq!(
        choice_from_reply(&ex.forecast, Ok(&body.to_string())).source,
        "model"
    );
    body["counter_offer"]["accepted_by"] = serde_json::json!(177);
    assert_eq!(
        choice_from_reply(&ex.forecast, Ok(&body.to_string())).source,
        "fallback"
    );
    body["counter_offer"]["accepted_by"] = serde_json::json!(ex.merges);
    body["counter_offer"]["wait_s"] = serde_json::json!(60.0);
    assert_eq!(
        choice_from_reply(&ex.forecast, Ok(&body.to_string())).source,
        "fallback"
    );
}

#[test]
fn valid_offer_delays_share_the_same_safety_governed_forecast() {
    use malleable::negotiate::{MergeChoice, MergeSnap, MergeTalk};
    struct Delay(f64);
    impl MergeTalk for Delay {
        fn agree(&self, snap: &MergeSnap) -> MergeChoice {
            let mut choice = ScriptedTalk.agree(snap);
            if let Some(offer) = &mut choice.counter_offer {
                offer.wait_s = self.0;
            }
            choice
        }
    }
    let fast = run_story_town(7, StoryKind::CounterOffer, Box::new(Delay(0.5)));
    let slow = run_story_town(7, StoryKind::CounterOffer, Box::new(Delay(3.0)));
    assert_eq!(
        fast.exchanges[0].activation_t,
        slow.exchanges[0].activation_t
    );
    assert_eq!(
        serde_json::to_value(&fast.frames).unwrap(),
        serde_json::to_value(&slow.frames).unwrap()
    );
}

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
