use muse::{
    incident::LaneIncident,
    network::BroadcastConfig,
    road_map::RoadMap,
    rush_hour::{self, Decision, Fcfs, Policy, PolicyRequest, Trip, Truth},
};
use std::sync::Mutex;

fn map() -> RoadMap {
    serde_json::from_str(r#"{"version":1,"origin":[0,0],"nodes":[
      {"id":0,"lat":0,"lon":0,"demand_per_s":0.2},
      {"id":1,"lat":0,"lon":0.002},
      {"id":2,"lat":0,"lon":0.004,"sink_weight":1}],"edges":[
      {"id":0,"from":0,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"},
      {"id":1,"from":1,"to":2,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"}]}"#).unwrap()
}
fn incident(start: f64, duration: f64, position: f64) -> LaneIncident {
    LaneIncident {
        version: 1,
        id: "test-obstacle".into(),
        kind: "blocked_lane".into(),
        map: "synthetic".into(),
        edge: 0,
        from_node: 0,
        to_node: 1,
        lane: 0,
        position_m: position,
        length_m: 8.0,
        start_s: start,
        duration_s: duration,
        rationale: "Test a fixed physical closure on a synthetic lane".into(),
    }
}
fn trip(car: u32, depart: f64) -> Trip {
    Trip {
        car,
        rider: car + 50000,
        route: 0,
        depart_s: depart,
        request_s: depart,
        reason: "I am commuting".into(),
        truth: Truth::Ordinary,
        destination: "other".into(),
        claim_number: 1,
    }
}

#[test]
fn cars_stop_for_the_obstacle_and_resume_after_its_fixed_end() {
    let mut net = map().network().unwrap();
    net.add_incident(incident(0.0, 60.0, 80.0)).unwrap();
    let run =
        rush_hour::simulate_trips_on(1, &Fcfs, true, None, vec![trip(17, 0.0)], &net).unwrap();
    assert_eq!(run.home.len(), 1);
    assert!(run.home[&17] > 60.0);
    for f in run.town.frames.iter().filter(|f| f.t < 60) {
        for c in &f.cars {
            assert!(c.2 < 80.0, "car crossed the active obstacle at t={}", f.t);
        }
    }
    let stopped = run.town.frames[30].cars.iter().find(|c| c.0 == 17).unwrap();
    assert!(stopped.3 < 0.1 && stopped.2 > 70.0);
    let before = run.town.frames[59].cars[0].2;
    let after = run.town.frames[60].cars[0].2;
    assert!(
        (after - before).abs() < 1.0,
        "incident expiry must not teleport the car"
    );
    assert_eq!(
        run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        0
    );
    assert_eq!(run.town.incidents.len(), 1);
}

#[test]
fn activation_allows_existing_occupants_to_clear_but_stops_followers() {
    let mut net = map().network().unwrap();
    net.add_incident(incident(2.0, 30.0, 20.0)).unwrap();
    let run = rush_hour::simulate_trips_on(
        1,
        &Fcfs,
        true,
        None,
        vec![trip(17, 0.0), trip(18, 4.0)],
        &net,
    )
    .unwrap();
    let first = run.town.frames[3].cars.iter().find(|c| c.0 == 17).unwrap();
    assert!(first.2 > 28.0 && first.3 > 5.0);
    let next = run.town.frames[20].cars.iter().find(|c| c.0 == 18).unwrap();
    assert!(next.2 < 20.0 && next.3 < 0.1);
    assert_eq!(run.home.len(), 2);
    assert_eq!(
        run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        0
    );
}

#[test]
fn invalid_and_duplicate_physical_inputs_are_rejected() {
    let mut net = map().network().unwrap();
    let mut bad = incident(0.0, 30.0, 190.0);
    assert!(net
        .add_incident(bad.clone())
        .unwrap_err()
        .contains("footprint"));
    bad.position_m = 50.0;
    bad.to_node = 2;
    assert!(net
        .add_incident(bad.clone())
        .unwrap_err()
        .contains("directed road"));
    bad.to_node = 1;
    bad.lane = 1;
    assert!(net.add_incident(bad.clone()).is_err());
    bad.lane = 0;
    bad.duration_s = f64::NAN;
    assert!(net.add_incident(bad).is_err());
    let good = incident(0.0, 30.0, 50.0);
    net.add_incident(good.clone()).unwrap();
    assert!(net.add_incident(good).unwrap_err().contains("duplicate"));
    let frozen = LaneIncident::read("scenarios/sf-potrero-incident.json").unwrap();
    let mut sf = RoadMap::read("maps/sf-zsfg.json")
        .unwrap()
        .network()
        .unwrap();
    sf.add_incident(frozen).unwrap();
}

struct Capture(Mutex<Vec<PolicyRequest>>);
impl Policy for Capture {
    fn name(&self) -> &str {
        "capture"
    }
    fn decide(&self, r: &PolicyRequest) -> Decision {
        self.0.lock().unwrap().push(r.clone());
        Fcfs.decide(r)
    }
}
#[test]
fn incident_metadata_never_enters_the_model_request() {
    let plain = map().network().unwrap();
    let mut blocked = plain.clone();
    blocked.add_incident(incident(0.0, 60.0, 80.0)).unwrap();
    let a = Capture(Mutex::new(Vec::new()));
    let b = Capture(Mutex::new(Vec::new()));
    let trips = vec![trip(17, 0.0), trip(18, 4.0)];
    rush_hour::simulate_trips_on(1, &a, true, None, trips.clone(), &plain).unwrap();
    rush_hour::simulate_trips_on(
        1,
        &b,
        true,
        Some(BroadcastConfig::default()),
        trips,
        &blocked,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&*a.0.lock().unwrap()).unwrap(),
        serde_json::to_vec(&*b.0.lock().unwrap()).unwrap()
    );
}

#[test]
fn a_grant_cannot_bypass_the_block_and_all_lane_changes_remain_physical() {
    let mut net = map().network().unwrap();
    net.add_incident(incident(0.0, 150.0, 140.0)).unwrap();
    let mut trips: Vec<_> = (0..9).map(|i| trip(100 + i, i as f64 * 0.5)).collect();
    let mut emergency = trip(999, 8.0);
    emergency.truth = Truth::Emergency;
    emergency.reason = "My passenger has severe chest pain".into();
    emergency.destination = "hospital".into();
    trips.push(emergency);
    let run = rush_hour::simulate_trips_on(
        1,
        &rush_hour::Mock,
        true,
        Some(BroadcastConfig::default()),
        trips,
        &net,
    )
    .unwrap();
    assert!(run.decisions.iter().any(|d| d.car == 999 && d.granted));
    assert!(run.messages.iter().any(|m| m.action == "lane_change"));
    assert!(run.home[&999] > 150.0);
    for frame in run.town.frames.iter().filter(|f| f.t < 150) {
        for car in &frame.cars {
            let mut start = 0.0;
            for link in &run.town.routes[car.1 as usize] {
                if *link == 0 {
                    assert!(
                        car.2 <= start + 140.0
                            || car.2 - muse::world::CAR_LENGTH_M >= start + 148.0,
                        "car {} occupied the blocked original lane at t={}",
                        car.0,
                        frame.t
                    );
                }
                start += run.town.links[*link].len;
            }
        }
    }
    assert_eq!(
        run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        0
    );
}

#[test]
fn incident_metadata_roundtrips_and_bad_replay_references_fail() {
    let mut net = map().network().unwrap();
    net.add_incident(incident(0.0, 30.0, 80.0)).unwrap();
    let run =
        rush_hour::simulate_trips_on(1, &Fcfs, true, None, vec![trip(17, 0.0)], &net).unwrap();
    let scored = rush_hour::ScoredRun {
        score: rush_hour::score(1, "fcfs", &run, &run).unwrap(),
        run,
    };
    let archive =
        muse::replay::ReplayArchive::new(muse::rush_board::from_run(&scored, None).unwrap());
    let path = std::env::temp_dir().join(format!("muse-incident-{}.json", std::process::id()));
    archive.write(&path).unwrap();
    let again = muse::replay::ReplayArchive::read(&path).unwrap();
    assert_eq!(
        serde_json::to_vec(&archive).unwrap(),
        serde_json::to_vec(&again).unwrap()
    );
    let mut invalid = again;
    invalid.board.town.incidents[0].link = usize::MAX;
    assert!(invalid
        .validate()
        .unwrap_err()
        .contains("invalid physical incident"));
    std::fs::remove_file(path).unwrap();
    let invalid_cli = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args([
            "--rush-hour",
            "--incident",
            "scenarios/sf-potrero-incident.json",
            "--policy",
            "mock",
        ])
        .output()
        .unwrap();
    assert!(!invalid_cli.status.success());
    assert!(String::from_utf8_lossy(&invalid_cli.stderr).contains("--incident needs --map"));
}
