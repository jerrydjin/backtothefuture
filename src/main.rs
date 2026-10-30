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

    if args.reason.is_some()
        || (args.board
            && !args.demo
            && args.priority.is_none()
            && args.story.is_none()
            && args.replay.is_none()
            && args.save_replay.is_none())
    {
        return match run_reason(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("reason: {error}");
                ExitCode::from(1)
            }
        };
    }

    if args.demo {
        return match muse::demo::run(
            args.headless,
            !args.no_open,
            args.require_live,
            args.include_weak,
            args.gate_log.as_deref(),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("demo: {error}");
                ExitCode::from(1)
            }
        };
    }

    if let Some(path) = &args.replay {
        let archive = match muse::replay::ReplayArchive::read(path) {
            Ok(archive) => archive,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
        println!("Replay: saved trajectories and dialogue; model calls=0");
        for strip in archive.board.strips.iter().filter(|s| s.scoreboard) {
            println!("{:<12}{}", strip.name, strip.score);
        }
        if let Some(path) = &args.save_replay {
            if let Err(error) = archive.write(path) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
        return match muse::board_server::serve_replay(&archive.board, !args.no_open) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("replay: {error}");
                ExitCode::from(1)
            }
        };
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
    if args.board || args.priority.is_some() || args.story.is_some() || args.save_replay.is_some() {
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
        if let Some(path) = &args.save_replay {
            if let Err(error) = muse::replay::ReplayArchive::new(board.clone()).write(path) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            println!("replay: {} (updated as replies arrive)", path.display());
        }
        if args.board {
            if let Err(error) = muse::board_server::serve_board_recording(
                &board,
                negotiator_from_env(),
                !args.no_open,
                args.save_replay.as_deref(),
            ) {
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
    demo: bool,
    headless: bool,
    require_live: bool,
    include_weak: bool,
    gate_log: Option<std::path::PathBuf>,
    no_open: bool,
    priority: Option<bool>,
    story: Option<muse::story::StoryKind>,
    replay: Option<std::path::PathBuf>,
    save_replay: Option<std::path::PathBuf>,
    rewrite_every: Option<u32>,
    reason: Option<muse::reason::ReasonText>,
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
            demo: false,
            headless: false,
            require_live: false,
            include_weak: false,
            gate_log: None,
            no_open: false,
            priority: None,
            story: None,
            replay: None,
            save_replay: None,
            rewrite_every: None,
            reason: None,
        };
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => parsed.help = true,
                "--show" => parsed.show = true,
                "--board" => parsed.board = true,
                "--reason" => {
                    let text = args.next().ok_or("--reason needs a sentence")?;
                    parsed.reason =
                        Some(muse::reason::ReasonText::new(&text).map_err(str::to_string)?);
                }
                "--demo" => parsed.demo = true,
                "--headless" => parsed.headless = true,
                "--require-live" => parsed.require_live = true,
                "--include-weak" => parsed.include_weak = true,
                "--gate-log" => {
                    parsed.gate_log = Some(args.next().ok_or("--gate-log needs a file")?.into())
                }
                "--no-open" => parsed.no_open = true,
                "--priority" => parsed.priority = Some(true),
                "--priority-baseline" => parsed.priority = Some(false),
                "--replay" => {
                    parsed.replay = Some(args.next().ok_or("--replay needs a file")?.into())
                }
                "--save-replay" => {
                    parsed.save_replay =
                        Some(args.next().ok_or("--save-replay needs a file")?.into())
                }
                "--story" => {
                    let value = args.next().ok_or(
                        "--story needs competing, abuse, emergency, chain, or counter-offer",
                    )?;
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
        if parsed.replay.is_some() && (parsed.story.is_some() || parsed.priority.is_some()) {
            return Err("--replay uses saved decisions; choose a replay or a new story".into());
        }
        if (parsed.headless || parsed.require_live || parsed.gate_log.is_some())
            && !parsed.demo
            && parsed.reason.is_none()
        {
            return Err(
                "--headless, --require-live, and --gate-log need --demo or --reason".into(),
            );
        }
        if parsed.reason.is_some()
            && (parsed.demo
                || parsed.priority.is_some()
                || parsed.story.is_some()
                || parsed.replay.is_some()
                || parsed.save_replay.is_some())
        {
            return Err("--reason uses a fresh trip; choose it separately from demo, priority, story, or replay".into());
        }
        if parsed.reason.is_some() && parsed.board && parsed.headless {
            return Err("choose --board or --headless for a reason".into());
        }
        if parsed.include_weak && !(parsed.demo && parsed.headless) {
            return Err("--include-weak needs --demo --headless".into());
        }
        if parsed.demo
            && (parsed.story.is_some()
                || parsed.priority.is_some()
                || parsed.replay.is_some()
                || parsed.save_replay.is_some())
        {
            return Err(
                "--demo plays the complete story; use story/replay flags separately".into(),
            );
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
  --demo                   play the finite normal/merge/priority/competing story
  --headless               drive the same demo over HTTP without a browser
  --require-live           fail the headless gate unless every reply is Baseten
  --include-weak           also gate the weak-request grant and repeated refusals
  --gate-log FILE          append the headless run report as JSONL
  --cars N                 fleet size, 2 to 64 (default 6)
  --jam KIND               symmetric | ramp-surge | mainline-plug
  --prefs SENTENCE         plain English merge preference
  --reason SENTENCE        assess a rider reason (1–240 characters); --board shows the trip
  --rewrite-every N        ask for a new plan every N ticks
  --show                   print the ramp merge in the terminal: wreck, held jam, and cleared road
  --board                  open the live birds-eye window: white network, flat cars, one yield or merge cue
  --no-open                print the board URL without opening a window (with --board)
  --priority               car 177 requests priority; cars ahead clear its path
  --priority-baseline      the same trip without its priority request
  --story KIND             competing | abuse | emergency | chain | counter-offer
  --save-replay FILE       save trajectories, dialogue, reasons, and reply times
  --replay FILE            play that saved result with zero model calls
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

fn run_reason(args: &Args) -> Result<(), String> {
    use std::io::Write;
    let baseline = muse::reason::baseline();
    let agent = muse::reason::ReasonAgent::from_env();
    let (town, report) = if let Some(reason) = &args.reason {
        let (town, mut report) = muse::reason::evaluate(reason, &agent, &baseline)?;
        report.passed &= !args.require_live
            || (report.decision.source == "model" && report.decision.provider == "baseten");
        let json = serde_json::to_string(&report).map_err(|_| "cannot serialize reason report")?;
        println!("reason-gate {json}");
        if let Some(path) = &args.gate_log {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            writeln!(file, "{json}").map_err(|e| e.to_string())?;
        }
        if args.require_live
            && (report.decision.source != "model" || report.decision.provider != "baseten")
        {
            return Err(
                "reason live gate requires a Baseten/model decision; fallback was not counted"
                    .into(),
            );
        }
        (town, Some(report))
    } else {
        (baseline, None)
    };
    let board = muse::reason::board(town, report)?;
    for strip in board.strips.iter().filter(|s| s.scoreboard) {
        println!("{:<12}{}", strip.name, strip.score);
    }
    if args.board {
        let server =
            muse::reason_server::ReasonServer::start(board, agent).map_err(|e| e.to_string())?;
        let url = format!("http://{}/?reason=1", server.address);
        println!("board: {url}");
        if !args.no_open {
            let _ = muse::board::open_board_url(&url);
        }
        loop {
            std::thread::park_timeout(std::time::Duration::from_secs(1));
        }
    }
    Ok(())
}
