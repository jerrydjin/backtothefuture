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
    assert_eq!(
        serde_json::to_string(&a.exchange).unwrap(),
        serde_json::to_string(&b.exchange).unwrap()
    );
}

#[test]
fn one_merge_speaks_and_the_yielder_waits() {
    let run = run_town(7);
    let ex = run.exchange.as_ref().expect("one exchange at one merge");
    assert_eq!(ex.source, "fallback");
    assert_eq!(ex.reason, "no_key");
    assert_eq!(ex.lines.len(), 2);
    assert_ne!(ex.yields, ex.merges);
    assert!(ex.until + 1e-6 >= ex.t);
    let log = ex.log();
    assert!(log.contains("source=fallback reason=no_key"), "{log}");
    assert!(!log.contains("sk-"), "{log}");
    assert!(log.contains(&format!("yields={}", ex.yields)), "{log}");
    assert!(log.contains(&format!("merges={}", ex.merges)), "{log}");
    for line in &ex.lines {
        let words = line.say.split_whitespace().count();
        assert!((1..=6).contains(&words), "{}", line.say);
        assert!(log.contains(&line.say), "{log}");
    }
    let along = |frame: &muse::network::NetFrame, id: u32| {
        frame
            .cars
            .iter()
            .find(|car| car.0 == id)
            .map(|car| (car.2, car.3))
    };
    let mut first = None;
    let mut last = None;
    let mut slow = false;
    for frame in &run.frames {
        let t = f64::from(frame.t);
        if t + 0.01 < ex.t || t > ex.until + 0.01 {
            continue;
        }
        if let (Some((y, yv)), Some((m, _))) = (along(frame, ex.yields), along(frame, ex.merges)) {
            if yv < 2.0 {
                slow = true;
            }
            if first.is_none() {
                first = Some((y, m));
            }
            last = Some((y, m));
        }
    }
    let (y0, m0) = first.expect("both cars are on the board when they speak");
    let (y1, m1) = last.expect("both cars stay through the yield");
    let y_delta = y1 - y0;
    let m_delta = m1 - m0;
    assert!(
        slow && m_delta > y_delta + 1.0,
        "yield follows the decision: yielder moved {y_delta:.1} (slow={slow}) merger moved {m_delta:.1}\n{log}"
    );
}
