use muse::make_way::{
    DrivingAction as A, FleetReceiver, MakeWayRequest, SafetyFleet, VehicleProposal,
};
fn request() -> MakeWayRequest {
    MakeWayRequest {
        id: "request-one".into(),
        issuer: 1,
        route: vec![0, 1],
        urgency: 1.0,
        reason: "Emergency precedence".into(),
        issued_at: 0.0,
        expires_at: 10.0,
        signature: None,
    }
}
fn proposal(action: A) -> VehicleProposal {
    VehicleProposal {
        vehicle: 2,
        action,
        hold_used_s: 0.0,
    }
}
#[test]
fn expired_and_not_yet_issued_requests_are_ignored() {
    let fleet = SafetyFleet(|_, _: &A| true);
    for at in [-1.0, 10.0, 11.0, f64::NAN] {
        assert_eq!(
            fleet.receive(&request(), at, &[proposal(A::Hold { duration_s: 1.0 })]),
            vec![(2, A::Ignore)]
        );
    }
}
#[test]
fn physics_rejection_cannot_be_overridden_by_a_fleet_action() {
    let fleet = SafetyFleet(|_, _: &A| false);
    for action in [
        A::LaneChange { to_link: 1 },
        A::Reroute(vec![0, 2]),
        A::Hold { duration_s: 1.0 },
    ] {
        assert_eq!(
            fleet.receive(&request(), 1.0, &[proposal(action)]),
            vec![(2, A::Ignore)]
        );
    }
}
#[test]
fn hold_is_capped_including_cumulative_time_and_invalid_values() {
    let fleet = SafetyFleet(|_, _: &A| true);
    let mut p = proposal(A::Hold { duration_s: 100.0 });
    p.hold_used_s = 2.75;
    assert_eq!(
        fleet.receive(&request(), 1.0, &[p.clone()]),
        vec![(2, A::Hold { duration_s: 0.25 })]
    );
    p.hold_used_s = 3.0;
    assert_eq!(
        fleet.receive(&request(), 1.0, &[p.clone()]),
        vec![(2, A::Ignore)]
    );
    p.action = A::Hold {
        duration_s: f64::NAN,
    };
    assert_eq!(fleet.receive(&request(), 1.0, &[p]), vec![(2, A::Ignore)]);
}
#[test]
fn ignore_is_always_allowed_and_every_vehicle_has_one_ordered_action() {
    let fleet = SafetyFleet(|_, _: &A| false);
    let p = [
        proposal(A::Ignore),
        VehicleProposal {
            vehicle: 3,
            ..proposal(A::Ignore)
        },
    ];
    assert_eq!(
        fleet.receive(&request(), 1.0, &p),
        vec![(2, A::Ignore), (3, A::Ignore)]
    );
}
#[test]
fn message_wire_and_delivery_log_round_trip_and_replay() {
    let run = muse::rush_hour::run_experiment(
        1,
        &muse::rush_hour::Mock,
        true,
        Some(muse::network::BroadcastConfig::default()),
    )
    .unwrap();
    let log = run.run.message_log;
    assert!(!log.requests.is_empty());
    let encoded = serde_json::to_vec(&log).unwrap();
    let restored: muse::make_way::MessageLog = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.replay().unwrap()).unwrap(),
        serde_json::to_vec(&log.events).unwrap()
    );
    assert_eq!(encoded, serde_json::to_vec(&restored).unwrap());
}

#[test]
fn replay_drops_scheduled_deliveries_after_revocation() {
    let mut req = request();
    req.expires_at = 0.0;
    let log = muse::make_way::MessageLog {
        version: 1,
        requests: vec![req],
        events: vec![muse::network::MessageHop {
            beneficiary: 1,
            route: 0,
            at_s: 1.0,
            route_m: 0.0,
            ahead_m: 0.0,
            lead_s: 0.0,
            sender: "car:1".into(),
            recipient: "car:2".into(),
            recipient_car: Some(2),
            connector: None,
            action: "relay".into(),
        }],
    };
    assert!(log.replay().unwrap().is_empty());
}
