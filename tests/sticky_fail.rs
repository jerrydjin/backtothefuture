use std::cell::Cell;

use muse::{
    default_horizon, interpret_english, judge, rollout, run_episode, validate_plan, Bias,
    DumbEqual, EpisodeOpts, FakeEcho, FollowPlan, JamType, LaneIntent, MuseClient, OrderIntent,
    Prefs, Scenario, Selfish, Verdict, World, ZonePlan, FAIRNESS_MAX,
};

const FAIR: &str = "Zipper fairly so both approaches get home.";

struct CountingClient {
    hits: Cell<usize>,
}

impl MuseClient for CountingClient {
    fn complete(&self, prompt: &str) -> String {
        self.hits.set(self.hits.get() + 1);
        FakeEcho.complete(prompt)
    }
}

fn fair(cars: u32, jam: JamType) -> Scenario {
    Scenario::new(cars, jam, FAIR)
}

#[test]
fn sticky_fail_baseline_red_and_muse_green() {
    for cars in [2, 3, 4, 6, 8] {
        for jam in [
            JamType::Symmetric,
            JamType::RampSurge,
            JamType::MainlinePlug,
        ] {
            let report = run_episode(
                &fair(cars, jam),
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
                "{cars} {jam} selfish should wreck the gate: {}",
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
                "{cars} {jam} dumb-equal jams without a wreck: {}",
                report.dumb_equal
            );
            assert_eq!(
                report.muse.verdict,
                Verdict::Green,
                "{cars} {jam} muse {} plan {}",
                report.muse,
                report.plan_json
            );
            assert_eq!(report.muse.collisions, 0);
            assert!(report.muse.fairness <= FAIRNESS_MAX);
            assert_eq!(report.muse_calls, 1);
        }
    }
}

#[test]
fn selfish_ignores_a_broadcast_plan() {
    let scenario = fair(6, JamType::Symmetric);
    let world = World::spawn(&scenario).unwrap();
    let prompt = muse::render_prompt(&scenario.prefs, &world.snapshot());
    let plan = ZonePlan::parse(&FakeEcho.complete(&prompt)).unwrap();
    let mut seeded = world.clone();
    seeded.broadcast_plan(&plan);
    seeded.stub_peer_pings();
    let (count, has_plan, has_ping) = seeded.inbox_summary(0);
    assert!(count >= 2 && has_plan && has_ping);

    let selfish = judge(&rollout(seeded.clone(), &Selfish, default_horizon(6)));
    let dumb = judge(&rollout(seeded, &DumbEqual, default_horizon(6)));
    assert_eq!(selfish.verdict, Verdict::Red);
    assert!(selfish.collisions >= 1);
    assert_eq!(dumb.verdict, Verdict::Red);
    assert_eq!(dumb.cars_home, 0);
}

#[test]
fn priority_plan_gets_home_and_stays_unfair() {
    let scenario = Scenario::new(6, JamType::Symmetric, "Main has priority. Main first.");
    assert_eq!(interpret_english(&scenario.prefs.english), Bias::FavorMain);
    let report = run_episode(&scenario, &FakeEcho, "scripted", &EpisodeOpts::default()).unwrap();
    assert_eq!(report.selfish.verdict, Verdict::Red);
    assert_eq!(report.muse.collisions, 0);
    assert_eq!(report.muse.cars_home, report.muse.cars);
    assert!(report.muse.fairness > FAIRNESS_MAX, "{}", report.muse);
    assert_eq!(report.muse.verdict, Verdict::Red);
    assert_eq!(report.muse.reasons, vec!["unfair".to_string()]);
}

#[test]
fn structured_bias_in_the_prompt_overrides_fair_english() {
    let mut scenario = fair(6, JamType::Symmetric);
    scenario.prefs = Prefs::with_bias(FAIR, Bias::FavorRamp);
    let report = run_episode(&scenario, &FakeEcho, "scripted", &EpisodeOpts::default()).unwrap();
    assert_eq!(report.muse.verdict, Verdict::Red);
    assert_eq!(report.muse.reasons, vec!["unfair".to_string()]);
    let plan = ZonePlan::parse(&report.plan_json).unwrap();
    let first = plan.intents.iter().find(|intent| intent.rank == 0).unwrap();
    assert_eq!(first.approach, muse::Approach::Ramp);
}

#[test]
fn mid_run_rewrite_stays_green() {
    let client = CountingClient { hits: Cell::new(0) };
    let report = run_episode(
        &fair(6, JamType::Symmetric),
        &client,
        "scripted",
        &EpisodeOpts {
            rewrite_every: Some(3),
            ..EpisodeOpts::default()
        },
    )
    .unwrap();
    assert_eq!(report.muse.verdict, Verdict::Green, "{}", report.muse);
    assert!(client.hits.get() > 1, "rewrite should call Muse again");
}

#[test]
fn reversed_queue_is_rejected_and_brakes_are_rejected() {
    let world = World::spawn(&fair(4, JamType::Symmetric)).unwrap();
    let broken = ZonePlan {
        zone_id: "merge-1".into(),
        intents: vec![
            OrderIntent {
                rank: 0,
                agent_id: 1,
                approach: muse::Approach::Main,
                lane: LaneIntent::Through,
            },
            OrderIntent {
                rank: 1,
                agent_id: 0,
                approach: muse::Approach::Main,
                lane: LaneIntent::Through,
            },
            OrderIntent {
                rank: 2,
                agent_id: 2,
                approach: muse::Approach::Ramp,
                lane: LaneIntent::Through,
            },
            OrderIntent {
                rank: 3,
                agent_id: 3,
                approach: muse::Approach::Ramp,
                lane: LaneIntent::Through,
            },
        ],
    };
    assert!(validate_plan(&broken, &world).is_err());

    let raw = r#"{"zone_id":"merge-1","intents":[{"rank":0,"agent_id":0,"approach":"main","lane":"through","brake":0}]}"#;
    assert!(ZonePlan::parse(raw).is_err());
}

#[test]
fn show_dinner_path_reads_wreck_jam_and_clear() {
    let report = run_episode(
        &fair(6, JamType::Symmetric),
        &FakeEcho,
        "scripted",
        &EpisodeOpts {
            show: true,
            ..EpisodeOpts::default()
        },
    )
    .unwrap();
    assert_eq!(
        report.selfish.to_string(),
        "RED time_home=inf collisions=1 fairness=1.00 home=0/6 (collision, not all home)"
    );
    assert_eq!(
        report.dumb_equal.to_string(),
        "RED time_home=inf collisions=0 fairness=1.00 home=0/6 (not all home)"
    );
    assert_eq!(
        report.muse.to_string(),
        "GREEN time_home=69 collisions=0 fairness=0.00 home=6/6"
    );

    let selfish = report.selfish_trace.expect("selfish trace");
    assert!(
        selfish.starts_with("wreck stays in the gore — RED "),
        "{selfish}"
    );
    assert!(selfish.contains("GATE WRECK"), "{selfish}");
    assert!(selfish.contains("WRECK:"), "{selfish}");
    assert!(selfish.contains('X'), "{selfish}");

    let dumb = report.dumb_trace.expect("dumb trace");
    assert!(
        dumb.starts_with("jam stays stopped at the gore — RED "),
        "{dumb}"
    );
    assert!(dumb.contains("jam holds for 64s"), "{dumb}");
    assert!(dumb.contains("JAM: both mouths stopped"), "{dumb}");
    assert!(dumb.contains("0/6 home"), "{dumb}");

    let muse = report.muse_trace.expect("muse trace");
    assert!(
        muse.starts_with("road clears; every car is home — GREEN "),
        "{muse}"
    );
    assert!(muse.contains("CLEAR: every car is home (6/6)"), "{muse}");
    assert!(muse.contains("home=6/6"), "{muse}");
}

#[test]
fn show_unfair_clear_does_not_print_green() {
    let scenario = Scenario::new(6, JamType::Symmetric, "Main has priority. Main first.");
    let report = run_episode(
        &scenario,
        &FakeEcho,
        "scripted",
        &EpisodeOpts {
            show: true,
            ..EpisodeOpts::default()
        },
    )
    .unwrap();
    assert_eq!(report.muse.verdict, Verdict::Red);
    assert_eq!(report.muse.reasons, vec!["unfair".to_string()]);
    let muse = report.muse_trace.expect("muse trace");
    assert!(
        muse.starts_with("cars reach home; fairness keeps the board red — RED "),
        "{muse}"
    );
    assert!(muse.contains("CLEAR: every car is home (6/6)"), "{muse}");
    assert!(
        !muse.contains("road clears; every car is home — GREEN"),
        "{muse}"
    );
}

#[test]
fn follow_plan_without_a_broadcast_does_not_clear_the_jam() {
    let scenario = fair(4, JamType::Symmetric);
    let world = World::spawn(&scenario).unwrap();
    let score = judge(&rollout(world, &FollowPlan, default_horizon(4)));
    assert_eq!(score.verdict, Verdict::Red);
    assert_eq!(score.cars_home, 0);
}
