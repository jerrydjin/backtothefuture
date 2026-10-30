# Priority request (JERRY-143)

```bash
cargo test
cargo run -- --board --priority
cargo run -- --board --priority-baseline
# No display: add --no-open and open the printed loopback URL.
# Optional live host: BASETEN_API_KEY=... cargo run -- --board --priority
```

`--priority` is the symbolic person-requested trigger. In the fixed seed-7 town,
car 177 requests priority at recording second 200. A blue car and “priority,
please” identify the request. Cars ahead relay it at two junctions, agreeing
which stream yields. Their short lines lead the visible movement, then fade.
The requested car reaches its existing destination; a small home label shows
the measured improvement. The board remains one minimalist white SVG network.

The browser requests each agreement about 25 real seconds ahead of its encounter.
Calls may overlap; a completed replay never loses an earlier agreement. Keys stay
in Rust. A late response pauses that encounter with “negotiating…”. Missing keys,
transport failures, and unusable replies use the scripted exchange. Priority
replies must acknowledge the requested path; a contradictory choice is rejected
with `reason=parse` and uses the fallback. The model chooses symbolic precedence
and dialogue, while ordinary car-following, crossing claims, and exit-space
checks continue to move the cars. The request never changes speed limits or routes.

## Measured proof

Time-home below is simulated time **from the request**, rather than wall time or
whole-fleet dinner time. The paired worlds are identical through the request,
use the same seed, and retain the target's route. The ordinary one-off merge
story is disabled in both members of this priority experiment.

| Car 177 | Time-home | Recorded home time | Agreements | Collisions |
| --- | ---: | ---: | ---: | ---: |
| No request | 141.00 s | 341.00 s | 0 | 0 |
| Priority request | 126.50 s | 326.50 s | 2 | 0 |

The request saves **14.50 s (10.3%)**. Junctions 9 and 21 are on its route.
Cars 118 and 200 relay the request, yielding against cars 200 and 283 respectively.
Lane overlaps and crossing occupancy conflicts are checked at every 0.25 s
physics step, including the full recorded drain after the target gets home.

65 cargo tests pass: 38 library, 3 board, 5 negotiate, 5 network, 3 priority,
2 scale sweep, 9 sticky. Priority tests prove identical pre-request worlds, the
same target route, two distinct route junctions, a greater-than-10-second gain,
zero collisions, safe rejection of a contradictory model reply, retained
agreements when asynchronous replies overlap, and unchanged sticky scores.

Browser runs show request → two exchanges → two yields → fade → measured home,
with every API key unset and with a mock OpenAI-compatible server. Mock requests
took 19.050 s and 19.035 s; both returned `source=model`. Ordinary-board checks
also cover an on-time 19.032 s mock reply and a late 30.027 s mock reply. The late
case shows the backup cue, then the actual returned exchange and movement.
These are mock transport measurements. No Baseten key is configured in this
environment, so real Baseten latency and wording are unmeasured.

Sticky remains RED / RED / GREEN, `time_home=69`. `src/judge.rs` and
`src/schema.rs` are unchanged. This PR is stacked on
`cursor/town-merge-talk-96f5` (PR #11); the agent never merges it.

## Cuts

The trigger and target are a fixed symbolic demo, not emergency detection or an
arbitrary-car picker. No new dashboard, map panels, lights, pedestrians, speed
boosts, or model control of per-tick driving. Live Baseten proof needs a key.

These cuts describe PR #12. Steps 7–12 continue in [negotiation stories](negotiation-stories.md), including symbolic emergency detection, chain negotiation, counter-offers, and replay.
