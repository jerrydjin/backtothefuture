# Finite demo verification

Branch: `codex/negotiation-stories-7-12`, PR [#13](https://github.com/jerrydjin/backtothefuture/pull/13), stacked on #12 and #11. No merge.

`cargo run -- --demo` plays one continuous trajectory on the existing white SVG network. Normal traffic approaches a merge; two drivers agree on the offered turn; the yielding driver waits. A typed hospital request then clears a path. Hospital and meeting requests compete at the next encounter. Playback ends on “Hospital takes priority over a meeting.” It does not loop past that decision.

The browser and `--demo --headless` consume the same server clock and the same `POST /negotiate?event=…` / `GET /result?event=…` responses. The model honors the safe offered order; its speech must match the actual waiting and moving cars. Calls start at the beginning of the approach, twenty-five seconds before the speaking beat. Only a late response shows “negotiating…”. A bounded fallback prevents an absent response from hanging the story. Baseten gets the live call when its key is present; without that key the demo stays scripted, regardless of other provider keys.

Speech is capped at eight whitespace-separated words. Numeric characters, written numbers, units, “gap”/“meters”, and contradictory waiting/going roles are rejected. Invalid speech receives a short role-consistent fallback. No measurements are displayed in the conversation.

The expanded gate includes a second HTTP session showing a first weak request's short grant followed by both refusals. The ledger must grant exactly once, keep the original expiry, and produce exactly one negotiation. The expanded batch cannot start without a passing basic ten-run log. The public demo still ends competing.

## Evidence

- `cargo test`: 86 passing (44 library tests plus 42 integration tests). Tests passed after every commit. CI also passed on `3340bdd` and the final code commit `2ac0e53`.
- [Chromium report](gates/browser-demo.json): 103.143 seconds, three POSTs, three result GETs, no JavaScript errors. Asserted one SVG, both human lines, a visible yield, typed priority and simultaneous competing requests, hospital-wins final reason, and a frozen final frame.
- [Preliminary scripted run log](gates/demo-scripted-preliminary.jsonl): five passes before the final spoken-role refinement.
- [Official scripted run log](gates/demo-basic-scripted.jsonl): real wall-clock HTTP sessions; per-run lengths, source, exact driver lines, and reply timings. These are the unkeyed half only, not a complete ten-run gate.
- Sticky remains `RED / RED / GREEN`, with `time_home=69`. `src/judge.rs` and `src/schema.rs` are unchanged.

Final code: `2ac0e53b96459206dba74bcc69ea5b93eaf64c81`; binary SHA-256: `fdaf475f6588f5a7bbafe97097ddf2a1175fd1997881bfb4a72a8ccd54b45ed3`.

| Run | Mode | Length (s) | Resolved | Collisions | Live reply time |
| --- | --- | ---: | ---: | ---: | --- |
| 1 | Scripted, key unset | 102.322 | 3 | 0 | Not a live call |
| 2 | Scripted, key unset | 102.251 | 3 | 0 | Not a live call |
| 3 | Scripted, key unset | 102.215 | 3 | 0 | Not a live call |
| 4 | Scripted, key unset | 102.567 | 3 | 0 | Not a live call |
| 5 | Scripted, key unset | 103.089 | 3 | 0 | Not a live call |
| Live runs one through five | Blocked: key unavailable | — | — | — | Unmeasured |

All five final scripted runs ended on the hospital-over-meeting reason, made three POSTs, and retrieved all three results. The ten-run mixed gate is **incomplete**. The expanded ten-run gate was **not run**, as required until the basic ten pass.

## Live gate status

**Blocked, not passed.** This workspace has no `BASETEN_API_KEY`, and its outbound policy does not allow `inference.baseten.co`. Repository CI was also attempted: [real-time gate run](https://github.com/jerrydjin/backtothefuture/actions/runs/41061455472), after its green tests, reported `BLOCKED: BASETEN_API_KEY is missing. No live run was counted.` No genuine live reply time or sample live speech has been measured for this change. No mock is counted as live.

Provide either the workspace key plus host access, or the repository Actions secret `BASETEN_API_KEY`. The failed gate job can then be rerun; it runs five genuine Baseten sessions and five unkeyed sessions, and only after those pass starts the expanded ten. The commands are at the top of the README. Partial no-key evidence does not unlock the expanded batch.

## Commits for this request

- `fa3e3c6`: finite continuous demo, shared HTTP playback, README entry command; 83 tests.
- `fc37648`: human speech, eight-word/no-numbers/unit checks and correct spoken roles; 85 tests.
- `05c7695`: explicit Baseten-or-scripted demo selection; 85 tests.
- `c95cc23`: weak-request HTTP case, refusal clock and repeatable gate runner; 86 tests.
- `3340bdd`: optional CI gate with explicit commit marker; 86 tests.
- `2ac0e53`: reject courtesy phrases that reverse the turn; fingerprint every gate binary; 86 tests.
- Final evidence commit: saved real run logs, post-commit test counts, browser report, and the explicit live-access blocker; 86 tests.

Cut: rerouting, photoreal rendering, CARLA, V2X race, pixels, lights, pedestrians, and 3D. No extra stories were added to the public demo after competing priorities. Genuine live evidence and the dependent expanded ten-run batch remain pending credentials.
