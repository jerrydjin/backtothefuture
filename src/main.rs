#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

use muse::negotiate::negotiator_from_env;
use muse::{
    board_from_run, interpret_english, run_dinner, Bias, EpisodeOpts, JamType, MuseKind, Scenario,
    Verdict, DINNER_SENTENCE,
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
    if args.board || args.priority.is_some() || args.story.is_some() {
        let board = if let Some(kind) = args.story {
            muse::board::story_board_from_run(&dinner, muse.label(), kind)
        } else {
            match args.priority {
                Some(enabled) => {
                    muse::board::priority_board_from_run(&dinner, muse.label(), enabled)
                }
                None => board_from_run(&dinner, muse.label()),
            }
        };
        if let Some(story) = &board.town.story {
            for decision in &story.decisions {
                println!(
                    "priority car={} granted={} reason={}",
                    decision.claim.car_id, decision.granted, decision.explanation
                );
            }
        }
        if let Some(p) = &board.town.priority {
            if let (Some(time), Some(baseline)) = (p.time_home_s, p.baseline_time_home_s) {
                println!("priority car={} requested={} time_home={time:.2}s baseline={baseline:.2}s saved={:.2}s collisions={} negotiations={}",
                    p.request.car_id, p.enabled, baseline-time,
                    board.town.stats.overlaps + board.town.stats.crossing_conflicts, board.town.exchanges.len());
            }
        }
        if args.board {
            if let Err(error) =
                muse::board_server::serve_board(&board, negotiator_from_env(), !args.no_open)
            {
                eprintln!("board: {error}");
                return ExitCode::from(1);
            }
        }
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
    no_open: bool,
    priority: Option<bool>,
    story: Option<muse::story::StoryKind>,
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
            no_open: false,
            priority: None,
            story: None,
            rewrite_every: None,
        };
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => parsed.help = true,
                "--show" => parsed.show = true,
                "--board" => parsed.board = true,
                "--no-open" => parsed.no_open = true,
                "--priority" => parsed.priority = Some(true),
                "--priority-baseline" => parsed.priority = Some(false),
                "--story" => {
                    let value = args
                        .next()
                        .ok_or("--story needs competing, abuse, or emergency")?;
                    parsed.story = Some(
                        muse::story::StoryKind::parse(&value)
                            .ok_or_else(|| format!("unknown story {value}"))?,
                    );
                }
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
  --board                  open the live birds-eye window: white network, flat cars, one yield or merge cue
  --no-open                print the board URL without opening a window (with --board)
  --priority               car 177 requests priority; cars ahead clear its path
  --priority-baseline      the same trip without its priority request
  --story KIND             competing | abuse | emergency
  --help

Negotiation:
  BASETEN_API_KEY          one town merge speaks through Baseten. This key wins.
  GEMINI_API_KEY           the same merge through Gemini when BASETEN_API_KEY is unset.
  MERGE_API_KEY            generic key. MODEL_API_BASE selects that host.
  MODEL_API_MODEL          optional model override.
  A missing key, timeout, or a bad reply prints source=fallback and a short reason. A live call prints provider=baseten or provider=gemini. The reason is not the key.

Score:
  GREEN requires zero collisions, every car home, fairness <= 0.15, and time-home inside budget.
  Selfish and dumb-equal stay RED. A fair scripted plan can go GREEN.

Blocker:
  MODEL_API_KEY selects the HTTP client (curl). When it is unset, the scripted plan runs (ALLOW_CHEAT).
  With a key, the client label is muse. ALLOW_CHEAT=0 with no key fails closed."
    )
}
