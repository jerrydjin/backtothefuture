#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

use muse::{
    board_from_run, interpret_english, present_board, render_board_html, run_dinner, Bias,
    EpisodeOpts, JamType, MuseKind, Scenario, Verdict, DINNER_SENTENCE,
};

fn main() -> ExitCode {
    let args = match Args::from_env() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            eprintln!();
            eprintln!("{}", usage());
            return ExitCode::from(1);
        }
    };
    if args.help {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }

    let muse = match MuseKind::from_env() {
        Ok(muse) => muse,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "blocker: set MODEL_API_KEY for a real completion, or leave it unset with ALLOW_CHEAT=1 (the default) so the scripted plan can run"
            );
            return ExitCode::from(1);
        }
    };

    let scenario = Scenario::new(args.cars, args.jam, args.prefs.clone());
    let opts = EpisodeOpts {
        rewrite_every: args.rewrite_every,
        horizon: None,
        show: args.show,
    };
    let dinner = match run_dinner(&scenario, &muse, &opts) {
        Ok(dinner) => dinner,
        Err(error) => {
            eprintln!("episode failed: {error}");
            return ExitCode::from(1);
        }
    };
    let report = dinner.to_report(muse.label(), &opts);

    println!("Muse day-1");
    println!("{DINNER_SENTENCE}");
    println!();
    println!(
        "scenario: N={} jam={} client={}",
        report.cars, report.jam, report.client
    );
    println!("prefs: {}", report.prefs);
    println!("selfish     {}", report.selfish);
    println!("dumb-equal  {}", report.dumb_equal);
    println!("{:<12}{}", report.client, report.muse);
    println!("{:<12}{}", "negotiate", dinner.negotiate.score);
    println!("plan: {}", report.plan_json);
    println!("plan calls: {}", report.muse_calls);
    if let Some(trace) = &report.selfish_trace {
        println!("\nselfish trace\n{trace}");
    }
    if let Some(trace) = &report.dumb_trace {
        println!("\ndumb-equal jam\n{trace}");
    }
    if let Some(trace) = &report.muse_trace {
        println!("\n{} trace\n{trace}", report.client);
    }
    if args.board {
        let html = render_board_html(&board_from_run(&dinner, muse.label()));
        match present_board(&html) {
            Ok(path) => println!("board: {}", path.display()),
            Err(error) => {
                eprintln!("board: {error}");
                return ExitCode::from(1);
            }
        }
    }

    let baseline_red =
        report.selfish.verdict == Verdict::Red && report.dumb_equal.verdict == Verdict::Red;
    if !baseline_red {
        eprintln!("sticky fail broken: a dumb baseline left the red");
        return ExitCode::from(1);
    }
    if interpret_english(&report.prefs) == Bias::Balanced && report.muse.verdict != Verdict::Green {
        eprintln!("fair prefs did not go green");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

struct Args {
    help: bool,
    cars: u32,
    jam: JamType,
    prefs: String,
    show: bool,
    board: bool,
    rewrite_every: Option<u32>,
}

impl Args {
    fn from_env() -> Result<Self, String> {
        let mut parsed = Args {
            help: false,
            cars: 6,
            jam: JamType::Symmetric,
            prefs: "Zipper fairly so both approaches get home.".to_string(),
            show: false,
            board: false,
            rewrite_every: None,
        };
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => parsed.help = true,
                "--show" => parsed.show = true,
                "--board" => parsed.board = true,
                "--cars" => {
                    let value = args.next().ok_or("--cars needs a number")?;
                    parsed.cars = value
                        .parse()
                        .map_err(|_| format!("bad --cars value {value}"))?;
                }
                "--jam" => {
                    let value = args.next().ok_or("--jam needs a type")?;
                    parsed.jam =
                        JamType::parse(&value).ok_or_else(|| format!("bad --jam value {value}"))?;
                }
                "--prefs" => {
                    parsed.prefs = args.next().ok_or("--prefs needs a sentence")?;
                }
                "--rewrite-every" => {
                    let value = args.next().ok_or("--rewrite-every needs a number")?;
                    let every: u32 = value
                        .parse()
                        .map_err(|_| format!("bad --rewrite-every value {value}"))?;
                    parsed.rewrite_every = (every > 0).then_some(every);
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        if !(2..=64).contains(&parsed.cars) {
            return Err("cars must be from 2 to 64".into());
        }
        Ok(parsed)
    }
}

fn usage() -> String {
    format!(
        "\
{DINNER_SENTENCE}

Usage:
  cargo run -- --cars 6 --jam symmetric --prefs \"Zipper fairly so both approaches get home.\"

Knobs:
  --cars N                 fleet size, 2 to 64 (default 6)
  --jam KIND               symmetric | ramp-surge | mainline-plug
  --prefs SENTENCE         plain English merge preference
  --rewrite-every N        ask for a new plan every N ticks
  --show                   print the ramp merge in the terminal: wreck, held jam, and cleared road
  --board                  open the live birds-eye window: thin roads, cars, one yield or merge cue
  --help

Score:
  GREEN requires zero collisions, every car home, fairness <= 0.15, and time-home inside budget.
  Selfish and dumb-equal stay RED. A fair scripted plan can go GREEN.

Blocker:
  MODEL_API_KEY selects the HTTP client (curl). When it is unset, the scripted plan runs (ALLOW_CHEAT).
  With a key, the client label is muse. ALLOW_CHEAT=0 with no key fails closed."
    )
}
