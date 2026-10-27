use muse::network::{run_town, Network, TownSim, NET_DT_S, QUEUED_MPS};

#[test]
fn routes_follow_the_roads() {
    let net = Network::town();
    let routes = net.routes();
    assert!(routes.len() > 40, "{}", routes.len());
    for (_, _, route) in &routes {
        for pair in route.links.windows(2) {
            let (a, b) = (&net.links[pair[0]], &net.links[pair[1]]);
            match b.via {
                Some((from, _)) => {
                    assert_eq!(from, a.id, "a connector starts where its approach ends")
                }
                None => assert_eq!(
                    a.via.map(|(_, to)| to),
                    Some(b.id),
                    "a road follows a connector"
                ),
            }
        }
    }
}

#[test]
fn crossing_paths_never_hold_cars_at_once() {
    let net = Network::town();
    let mut sim = TownSim::new(3);
    for _ in 0..(300.0 / NET_DT_S) as u32 {
        sim.step();
        let bodies = sim.bodies();
        for &(id, link, _, _) in &bodies {
            if net.links[link].via.is_none() {
                continue;
            }
            for &(other, other_link, _, _) in &bodies {
                if other != id && net.conflicts_of(link).contains(&other_link) {
                    panic!("cars {id} and {other} share a junction at {}s", sim.time());
                }
            }
        }
        assert_eq!(sim.overlaps(), 0, "same-lane overlap at {}s", sim.time());
    }
}

#[test]
fn rush_hour_queues_form_and_drain() {
    for seed in [1, 7, 42] {
        let run = run_town(seed);
        let stats = &run.stats;
        assert_eq!(stats.overlaps, 0, "seed {seed}: {stats:?}");
        assert!(
            stats.peak_queued >= 15,
            "seed {seed}: no congestion {stats:?}"
        );
        assert!(
            stats.exited > 200,
            "seed {seed}: traffic should flow {stats:?}"
        );
        let queued = |frame: &muse::network::NetFrame| {
            frame.cars.iter().filter(|car| car.3 < QUEUED_MPS).count() as u32
        };
        let first = queued(&run.frames[0]);
        let last = queued(run.frames.last().expect("frames"));
        assert!(first * 3 < stats.peak_queued, "seed {seed}: starts light");
        assert!(
            last * 2 < stats.peak_queued,
            "seed {seed}: drains, {last} left of {stats:?}"
        );
    }
}

#[test]
fn the_town_is_deterministic() {
    let a = run_town(7);
    let b = run_town(7);
    assert_eq!(a.frames.len(), b.frames.len());
    let json = |run: &muse::network::NetworkRun| serde_json::to_string(&run.frames).expect("json");
    assert_eq!(json(&a), json(&b));
}
