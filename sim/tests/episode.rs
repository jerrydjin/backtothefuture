use traffic::episode::{episode, run, Jam, Scenario};
use traffic::judge::Verdict;
use traffic::muse::{FakeEcho, Planner};
use traffic::plan::{self, LaneIntent, ZonePlan};
use traffic::policy::{Greedy, PlanFollower};
use traffic::world::Lane;

const JAMS: [Jam; 3] = [Jam::Rush, Jam::RampHeavy, Jam::Wave];
const SIZES: [u32; 4] = [4, 8, 16, 40];

fn scenarios() -> impl Iterator<Item = Scenario> {
    JAMS.into_iter().flat_map(|jam| {
        SIZES.into_iter().flat_map(move |n| {
            [false, true]
                .into_iter()
                .map(move |ambulance| Scenario { n, jam, ambulance })
        })
    })
}

#[test]
fn selfish_baseline_always_red() {
    for s in scenarios() {
        let r = run(&s, &Greedy, false);
        assert_eq!(r.score.verdict, Verdict::Red, "{s:?}: {:?}", r.score);
        assert!(r.score.collisions > 0, "{s:?}");
    }
}

#[test]
fn fake_echo_plan_goes_green() {
    let prefs = [
        "Take turns fairly.",
        "Ambulance first, then take turns",
        "commuters on the main road first",
        "let the ramp in first, nobody waits more than 10 ticks",
    ];
    for s in scenarios() {
        for p in prefs {
            let ep = episode(&s, p, &FakeEcho, false);
            assert!(ep.error.is_none(), "{s:?} {p}: {:?}", ep.error);
            let planned = ep.planned.unwrap();
            assert_eq!(
                planned.score.verdict,
                Verdict::Green,
                "{s:?} {p}: {:?}",
                planned.score
            );
            assert_eq!(planned.score.collisions, 0);
            assert!(planned.score.mean_time_home < ep.baseline.score.mean_time_home);
        }
    }
}

#[test]
fn prefs_change_who_waits() {
    let s = Scenario {
        n: 30,
        jam: Jam::RampHeavy,
        ambulance: false,
    };
    let main_first = episode(&s, "main first", &FakeEcho, false)
        .planned
        .unwrap()
        .score;
    let ramp_first = episode(&s, "ramp first", &FakeEcho, false)
        .planned
        .unwrap()
        .score;
    assert!(main_first.mean_delay_main < ramp_first.mean_delay_main);
    assert!(main_first.mean_delay_ramp > ramp_first.mean_delay_ramp);
}

#[test]
fn ambulance_pref_gets_ambulance_home_sooner() {
    let s = Scenario {
        n: 24,
        jam: Jam::Rush,
        ambulance: true,
    };
    let plain = episode(&s, "take turns", &FakeEcho, false).planned.unwrap();
    let prio = episode(&s, "ambulance first", &FakeEcho, false)
        .planned
        .unwrap();
    assert!(prio.ambulance_time_home.unwrap() < plain.ambulance_time_home.unwrap());
}

#[test]
fn runs_are_deterministic() {
    let s = Scenario {
        n: 16,
        jam: Jam::Wave,
        ambulance: true,
    };
    let a = serde_json::to_string(&episode(&s, "ambulance first", &FakeEcho, true)).unwrap();
    let b = serde_json::to_string(&episode(&s, "ambulance first", &FakeEcho, true)).unwrap();
    assert_eq!(a, b);
}

#[test]
fn strict_priority_still_green_without_crashes() {
    let s = Scenario {
        n: 16,
        jam: Jam::Rush,
        ambulance: false,
    };
    let plan = ZonePlan {
        zone: "merge_a".into(),
        order: vec![
            LaneIntent {
                lane: Lane::Main,
                share: 1,
            },
            LaneIntent {
                lane: Lane::Ramp,
                share: 0,
            },
        ],
        priority_vehicles: vec![],
        max_wait: None,
        rationale: String::new(),
    };
    let r = run(&s, &PlanFollower { plan }, false);
    assert_eq!(r.score.collisions, 0);
    assert_eq!(r.score.home, 16);
}

fn plan_with(f: impl FnOnce(&mut ZonePlan)) -> ZonePlan {
    let mut p = ZonePlan {
        zone: "merge_a".into(),
        order: vec![
            LaneIntent {
                lane: Lane::Main,
                share: 1,
            },
            LaneIntent {
                lane: Lane::Ramp,
                share: 1,
            },
        ],
        priority_vehicles: vec![],
        max_wait: None,
        rationale: String::new(),
    };
    f(&mut p);
    p
}

#[test]
fn validation_rejects_bad_plans() {
    let state = Scenario {
        n: 8,
        jam: Jam::Rush,
        ambulance: false,
    }
    .spawn()
    .state();
    assert!(plan_with(|_| {}).validate(&state).is_ok());
    assert!(plan_with(|p| p.zone = "elsewhere".into())
        .validate(&state)
        .is_err());
    assert!(plan_with(|p| p.order.clear()).validate(&state).is_err());
    assert!(plan_with(|p| p.order[1].lane = Lane::Main)
        .validate(&state)
        .is_err());
    assert!(plan_with(|p| p.order.iter_mut().for_each(|a| a.share = 0))
        .validate(&state)
        .is_err());
    assert!(plan_with(|p| p.order[0].share = 99)
        .validate(&state)
        .is_err());
    assert!(plan_with(|p| p.max_wait = Some(0))
        .validate(&state)
        .is_err());
    assert!(plan_with(|p| p.priority_vehicles = vec![999])
        .validate(&state)
        .is_err());
}

#[test]
fn parse_reply_tolerates_chatter() {
    let reply = "Sure! Here is the plan:\n```json\n{\"zone\":\"merge_a\",\"order\":[{\"lane\":\"ramp\",\"share\":2}]}\n```";
    let p = plan::parse_reply(reply).unwrap();
    assert_eq!(p.order[0].lane, Lane::Ramp);
    assert!(plan::parse_reply("no plan here").is_err());
}

struct Broken;
impl Planner for Broken {
    fn name(&self) -> &str {
        "broken"
    }
    fn complete(&self, _: &str) -> Result<String, String> {
        Ok("{\"zone\":\"merge_a\",\"order\":[]}".into())
    }
}

#[test]
fn bad_plan_reports_error_and_keeps_baseline() {
    let s = Scenario {
        n: 8,
        jam: Jam::Rush,
        ambulance: false,
    };
    let ep = episode(&s, "anything", &Broken, false);
    assert!(ep.planned.is_none());
    assert!(ep.error.unwrap().contains("order is empty"));
    assert_eq!(ep.baseline.score.verdict, Verdict::Red);
}
