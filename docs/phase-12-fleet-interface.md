# Make-way fleet interface (wire version 1)

`src/make_way.rs` owns MakeWayRequest, FleetReceiver, SafetyFleet,
VehicleProposal, DrivingAction and MessageLog. The model supplies precedence
only; the host creates requests and its physics adapter proposes actions.
Simulation route actions and holds pass through SafetyFleet. Lane gaps and
stopping room are checked against current bodies serially; reroutes use the
host's legal graph and retain the current edge and original sink. The receiver
can reject every proposal. No receiver can override physics rejection.

An example JSON request (time in simulation seconds; route contains host edge
ids, including explicit intersection connector edges):

```json
{"id":"car:12@1","issuer":12,"route":[0,7,2],"urgency":1.0,"reason":"Host granted emergency precedence","issued_at":1.0,"expires_at":10001.0,"signature":null}
```

Requests are active on [issued_at, expires_at). Signature is an authentication
placeholder, not a claim of cryptographic verification. The simulator's normal
request lifetime is longer than its experiment horizon, preserving driving.
Explicit denial/revocation immediately expires that request, clears all of its
junction holds and releases pulled-over cars to normal physics on their current
lane. Release never forces a merge. Expiration is checked each tick too.
Hold is capped at the remaining cumulative 3-second budget. Invalid durations
and spent budgets return Ignore. Ignore never needs physics approval.

Actions serialize with an `action` discriminator and `parameters`, for example
`{"action":"hold","parameters":{"duration_s":0.25}}`; reroute parameters
are an array of edge ids. Every input vehicle receives one ordered action.

Save request/delivery evidence:

```sh
cargo run --release -- --map maps/town.json --scenario boxed-in --headless --policy mock --seed 11 --save-messages messages.json
```

MessageLog has version, requests and events. Deserialize it and call `replay()`
to replay recorded deliveries without new model calls. Motion replay is a
separate existing archive: recorded messages never authorize unsafe movement.
Round-trip and replay are tested, as are expiry, physics veto, cumulative hold
cap, ordered Ignore, and immediate revocation release. The unrelated-network
make-way integration test remains part of the suite.

| Gate | Target | Measured | Result |
|---|---|---|---|
| Driving scores vs main 255baa9, seeds 11–20 | byte-identical | 60/60 exact (boxed/typical × default/hold/lane) | PASS |
| Guarantee tests | every guarantee | expiry, veto, cap, revoke/release, Ignore, ordered output and replay covered | PASS |
| Second-network test | unchanged safety test passes | 1/1 | PASS |
| Cargo tests | no failures | 155 passed, 0 failed, 3 ignored | PASS |
| judge.rs/schema.rs | unchanged | 2/2 | PASS |

The full suite passed 154 tests; the final wire replay change passed all 6
fleet tests (5 repeated plus 1 new), for 155 distinct passing tests total.
Revoked requests can retain scheduled relay evidence; replay discards deliveries
at or after expiry. No live calls and no held-out seed tuning were performed.
