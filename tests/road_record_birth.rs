#[allow(dead_code)]
#[path = "../examples/phase15_birth.rs"]
mod birth;
use malleable::{
    incident::LaneIncident,
    network::Network,
    record::{Arm, Reach, RecordConfig, Reply, Scope},
    rush_hour::{self, Fcfs, Mock, Trip, Truth},
};
fn config() -> RecordConfig {
    RecordConfig {
        arm: Arm::Record,
        sensor_range_m: 300.0,
        proposal: None,
        reply: Reply::mock(),
        priority_reach: Reach {
            scope: Scope::Route,
            radius_m: 300.0,
        },
    }
}
fn trip(at: f64, reason: &str) -> Trip {
    Trip {
        car: 1,
        rider: 50001,
        route: 0,
        depart_s: at,
        request_s: at,
        reason: reason.into(),
        truth: Truth::Ordinary,
        destination: "other".into(),
        claim_number: 1,
    }
}
#[test]
fn birth_twins_preserve_every_trip_and_declared_cross_flow() {
    let w = birth::build().unwrap();
    assert!(!w.flows.is_empty());
    for seed in 1..=10 {
        let (a, probe) = birth::population(seed, &w, true);
        let (b, twin) = birth::population(seed, &w, false);
        assert_eq!(probe, twin);
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(
                (a.car, a.route, a.depart_s, a.request_s),
                (b.car, b.route, b.depart_s, b.request_s)
            );
            assert!(a.depart_s < 900.0);
            if a.car != probe {
                assert_eq!(a.reason, b.reason);
            }
        }
        assert_eq!(
            a.iter()
                .filter(|t| t.reason == "My partner is in active labour")
                .count(),
            1
        );
        assert_eq!(
            b.iter()
                .filter(|t| t.reason == "I'm late for the gym")
                .count(),
            1
        );
    }
}
#[test]
fn granted_priority_is_a_record_but_denied_gym_has_no_edit_or_action() {
    let n = Network::town();
    let (labour, s) = rush_hour::simulate_trips_recorded(
        1,
        &Mock,
        vec![trip(0.0, "My partner is in active labour")],
        &n,
        config(),
    )
    .unwrap();
    assert!(labour.decisions[0].granted);
    assert_eq!(s.priorities.len(), 1);
    assert_eq!(s.priorities[0].kind, "priority");
    assert_eq!(s.priorities[0].expires_at - s.priorities[0].since_s, 240.0);
    let (gym, s) = rush_hour::simulate_trips_recorded(
        1,
        &Mock,
        vec![trip(0.0, "I'm late for the gym")],
        &n,
        config(),
    )
    .unwrap();
    assert!(!gym.decisions[0].granted);
    assert!(s.priorities.is_empty());
    assert!(s.events.is_empty());
}
#[test]
fn unfinished_never_departed_never_entered_and_on_network_are_retained() {
    let n = Network::town();
    let (r, s) =
        rush_hour::simulate_trips_recorded(1, &Fcfs, vec![trip(3000.0, "commuting")], &n, config())
            .unwrap();
    assert_eq!(r.trips.len(), 1);
    assert!(r.home.is_empty());
    assert!(s.entered.is_empty());
    assert_eq!(s.ended_at_s, 2500.25);
    for position in [0.0, 20.0] {
        let mut n = Network::town();
        let source = n.routes()[0].2.links[0];
        let l = &n.links[source];
        n.add_incident(LaneIncident {
            version: 1,
            id: "test".into(),
            kind: "blocked_road".into(),
            map: "synthetic".into(),
            edge: l.edge,
            from_node: l.from,
            to_node: l.to,
            lane: 0,
            position_m: position,
            length_m: 8.0,
            start_s: 0.0,
            duration_s: 3000.0,
            rationale: "synthetic horizon audit".into(),
        })
        .unwrap();
        let (r, s) = rush_hour::simulate_trips_recorded(
            1,
            &Fcfs,
            vec![trip(0.0, "commuting")],
            &n,
            config(),
        )
        .unwrap();
        assert_eq!(r.trips.len(), 1);
        assert!(r.home.is_empty());
        if position == 0.0 {
            assert!(s.entered.is_empty());
            assert!(s.final_links.is_empty());
        } else {
            assert!(s.entered.contains_key(&1));
            assert!(s.final_links.contains_key(&1));
        }
    }
}
