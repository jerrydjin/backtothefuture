//! Fleet-size guard for the sticky board. `sticky_fail.rs` covers N up to 8;
//! this pins the same RED / RED / GREEN verdicts at the larger fleets a
//! network board will draw, and checks a rerun gives the same scores.

use muse::{run_episode, EpisodeOpts, FakeEcho, JamType, Scenario, Verdict, FAIRNESS_MAX};

const FAIR: &str = "Zipper fairly so both approaches get home.";
const JAMS: [JamType; 3] = [
    JamType::Symmetric,
    JamType::RampSurge,
    JamType::MainlinePlug,
];

#[test]
fn sticky_board_holds_at_fleet_scale() {
    for cars in [12, 24, 48, 64] {
        for jam in JAMS {
            let report = run_episode(
                &Scenario::new(cars, jam, FAIR),
                &FakeEcho,
                "scripted",
                &EpisodeOpts::default(),
            )
            .unwrap_or_else(|error| panic!("{cars} {jam}: {error}"));
            assert_eq!(
                report.selfish.verdict,
                Verdict::Red,
                "{cars} {jam} selfish {}",
                report.selfish
            );
            assert!(
                report.selfish.collisions >= 1,
                "{cars} {jam} selfish {}",
                report.selfish
            );
            assert_eq!(
                report.dumb_equal.verdict,
                Verdict::Red,
                "{cars} {jam} dumb-equal {}",
                report.dumb_equal
            );
            assert_eq!(
                report.dumb_equal.collisions, 0,
                "{cars} {jam} dumb-equal {}",
                report.dumb_equal
            );
            assert_eq!(
                report.muse.verdict,
                Verdict::Green,
                "{cars} {jam} muse {}",
                report.muse
            );
            assert_eq!(
                report.muse.cars_home, cars,
                "{cars} {jam} muse {}",
                report.muse
            );
            assert!(
                report.muse.fairness <= FAIRNESS_MAX,
                "{cars} {jam} muse {}",
                report.muse
            );
        }
    }
}

#[test]
fn reruns_score_identically() {
    for jam in JAMS {
        let run = || {
            let report = run_episode(
                &Scenario::new(48, jam, FAIR),
                &FakeEcho,
                "scripted",
                &EpisodeOpts::default(),
            )
            .unwrap_or_else(|error| panic!("48 {jam}: {error}"));
            format!(
                "{}|{}|{}|{}",
                report.selfish, report.dumb_equal, report.muse, report.plan_json
            )
        };
        assert_eq!(run(), run(), "48 {jam} is not deterministic");
    }
}
