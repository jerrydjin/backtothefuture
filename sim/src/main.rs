use std::process::ExitCode;
use traffic::episode::{episode, Jam, Run, Scenario};
use traffic::muse;

const USAGE: &str = "usage: traffic [--n N] [--jam rush|ramp_heavy|wave] [--ambulance] [--prefs TEXT] [--json] [--trace]

Runs one episode: selfish baseline, then one planner call, then the plan.
Uses Muse when MODEL_API_KEY (+ MODEL_API_BASE, MODEL_NAME) is set, else FakeEcho.";

fn main() -> ExitCode {
    let mut scenario = Scenario {
        n: 16,
        jam: Jam::Rush,
        ambulance: false,
    };
    let mut prefs = String::from("Take turns fairly at the merge.");
    let mut json = false;
    let mut trace = false;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        let res: Result<(), String> = match a.as_str() {
            "--n" => val("--n")
                .and_then(|v| v.parse().map_err(|e| format!("--n: {e}")))
                .map(|n| scenario.n = n),
            "--jam" => val("--jam")
                .and_then(|v| v.parse())
                .map(|j| scenario.jam = j),
            "--prefs" => val("--prefs").map(|p| prefs = p),
            "--ambulance" => {
                scenario.ambulance = true;
                Ok(())
            }
            "--json" => {
                json = true;
                Ok(())
            }
            "--trace" => {
                trace = true;
                Ok(())
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => Err(format!("unknown argument {other:?}")),
        };
        if let Err(e) = res {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }

    let planner = muse::from_env();
    let ep = episode(&scenario, &prefs, planner.as_ref(), trace);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ep).expect("episode serialises")
        );
    } else {
        println!(
            "scenario: n={} jam={:?} ambulance={}",
            scenario.n, scenario.jam, scenario.ambulance
        );
        println!("prefs:    {prefs}");
        println!("planner:  {}\n", ep.planner);
        if let Some(plan) = &ep.plan {
            println!(
                "plan:\n{}\n",
                serde_json::to_string_pretty(plan).expect("plan serialises")
            );
        }
        println!(
            "{:<17} {:>7} {:>10} {:>10} {:>9} {:>11} {:>11} {:>9}  verdict",
            "run",
            "home",
            "collisions",
            "mean_home",
            "max_home",
            "delay_main",
            "delay_ramp",
            "fairness"
        );
        row(&ep.baseline);
        if let Some(r) = &ep.planned {
            row(r);
        }
        if let Some(e) = &ep.error {
            println!("\nplanner failed: {e}");
        }
    }

    match &ep.planned {
        Some(r) if r.score.verdict == traffic::judge::Verdict::Green => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

fn row(r: &Run) {
    let s = &r.score;
    println!(
        "{:<17} {:>7} {:>10} {:>10.1} {:>9} {:>11.1} {:>11.1} {:>9.2}  {:?}{}",
        r.policy,
        format!("{}/{}", s.home, s.cars),
        s.collisions,
        s.mean_time_home,
        s.max_time_home,
        s.mean_delay_main,
        s.mean_delay_ramp,
        s.fairness,
        s.verdict,
        if s.reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", s.reasons.join("; "))
        },
    );
    if let Some(t) = r.ambulance_time_home {
        println!("{:<17} ambulance home in {t} ticks", "");
    }
}
