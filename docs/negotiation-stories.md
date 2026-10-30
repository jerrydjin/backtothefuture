# Negotiation stories: steps 7–12

Built on PR #12, on `codex/negotiation-stories-7-12` (PR #13).

```bash
cargo test
cargo run -- --board --story competing
cargo run -- --board --story abuse
cargo run -- --board --story emergency
cargo run -- --board --story chain
cargo run -- --board --story counter-offer --save-replay /tmp/traffic-replay.json
cargo run -- --replay /tmp/traffic-replay.json
```

Add `--no-open` on a host without a display and open its printed loopback URL.
The default requires no API key. `BASETEN_API_KEY` selects the existing live
adapter. Each agreement is requested about 25 real seconds ahead; its timeout
is 45 seconds. A late reply holds the encounter with “negotiating…”. The board
stays one continuous minimalist white SVG, with small on-car lines and reasons.

## What happens

| Step | Story | On-map behavior |
| --- | --- | --- |
| 7 | `competing` | Hospital and meeting riders ask at t=200. Hospital wins in either input order. The meeting car yields; one line explains “Hospital takes priority over a meeting.” |
| 8 | `abuse` | A meeting request gets one 30-second slot. The same rider's later weak requests get “No: repeated weak reasons don't earn priority.” They cannot extend the slot or buy the second junction. |
| 9 | `emergency` | No rider submits a request. Three consecutive symbolic distress samples spanning two seconds trigger priority automatically, with “distress detected · no request” over the car. |
| 10 | `chain` | The detected emergency is relayed by at least three actual cars. Every participant speaks and acknowledges; local queue order remains intact. |
| 11 | `counter-offer` | One car pushes back: let it cross, then hospital priority. The priority queue head explicitly accepts. The priority stream waits until the crossing physically clears; the board explains the accepted compromise. |
| 12 | replay | Save the actual trajectories, words, reasons, source labels, and reply times. Play the snapshot without a new simulation or model call. The small replay button or R starts it again. |

History follows the rider, not the car or request sequence. Duplicate requests
cannot grant a second slot. Repeated weak attempts require 120 seconds of quiet
before they can win again; stronger hospital urgency is still considered.
The sensor detector rejects single alarms, duplicate timestamps, false samples,
and gaps longer than 1.1 seconds. Its example observations are simulation data,
not hardware readings or rider-supplied urgency text.

The chain includes the requesting car and two real queue heads, with symbolic
positions and a shared cause in the prompt. Missing acknowledgements, an unknown
participant, or naming a relay as the mouth car that yields causes an explicit
scripted fallback. The model never gives a car a new route or speed command.

A counter-offer names its proposer, accepting queue head, bounded minimum wait
(0.5–3 seconds), and short explanation. Missing acceptance or an unbounded delay
is rejected. In the demo, the conversation starts at t=225.3 and hospital
precedence activates at t=230.5, after the accepted crossing clears. A timer
alone cannot authorize entry. Subsequent conversations leave enough room for
the previous words to fade before another exchange starts.

## Measured outcomes

These are simulated seconds **from the trigger at t=200**, rather than wall
latency or the whole-fleet dinner score. All use the same seed-7 fixture and the
target's original route. Both lane overlaps and crossing occupancy conflicts
are checked at every 0.25-second recorded physics step.

| Car 177 experiment | Time-home | Gain vs no request | Agreements | Collisions |
| --- | ---: | ---: | ---: | ---: |
| No request | 141.00 s | — | 0 | 0 |
| Hospital vs meeting | 126.50 s | 14.50 s | 2 | 0 |
| Repeated weak requests | 141.00 s | 0.00 s | 1 | 0 |
| Automatically detected emergency | 126.50 s | 14.50 s | 2 | 0 |
| Three-car emergency chain | 126.50 s | 14.50 s | 2 | 0 |
| Accepted counter-offer | 101.50 s | 39.50 s | 3 | 0 |

The counter-offer changes the order in which existing junction queues clear;
its gain is measured from the resulting trajectories, not a speed boost or
rerouting. Its valid minimum wait values share the same safety-governed
forecast, since the occupied crossing takes longer to clear than any of them.

## Replay and validation

`--save-replay FILE` writes a versioned board snapshot, then atomically updates
it as live replies arrive. Finish the story before reopening its final snapshot.
It contains no key or HTTP authorization header. `--replay FILE` loads saved
frames and decisions before any client initialization, disables the completion
endpoint, and makes zero model calls, even with unusable API keys and
`ALLOW_CHEAT=0`. Unsupported versions, broken routes, and missing participant
acknowledgements are rejected. Floating-point round trips preserve the exact
serialized geometry and trajectories. Replay text remains data in the page.

80 cargo tests pass: 39 library, 3 board, 5 negotiate, 5 network, 3 priority,
3 replay, 2 scale sweep, 9 sticky, 11 stories. Tests prove competing request
ordering, weak-request denials under changing request IDs and cars, bounded
physical slots, noisy-vs-sustained detection, actual chain participants,
complete acknowledgements, accepted and bounded counter-offers, physical
crossing order, collision-free runs, exact replay of all five stories, and
recording received model words rather than the precomputed fallback.

Browser checks cover the visible reasons, three speakers, physical yield phases,
no-key emergency chain, replay button, exact saved model dialogue, and a refused
reply poll that recovers without another model request. A full mock HTTP run
shows all three counter-offer agreements with `source=model`, reply times
19.030 s, 19.015 s, and 19.039 s, and the measured home result. Its archive plays
with zero completion requests and an absent completion endpoint. These are
mock transport measurements: no Baseten key is configured, so actual Baseten
wording and latency remain unmeasured.

| Step | Verified commit | Passing cargo tests |
| --- | --- | ---: |
| 7 | `c41b355` | 67 |
| 8 | `a12f1c2` | 69 |
| 9 | `e0fb375` | 71 |
| 10 | `f1314a0` | 73 |
| 11 | `ab6f642` | 76 |
| 12 | `f424b2b` | 80 |

Sticky stays RED / RED / GREEN, `time_home=69`. `src/judge.rs` and
`src/schema.rs` are untouched. No merge was performed. Rerouting remains cut;
rider identities and emergency observations remain symbolic simulation inputs.
