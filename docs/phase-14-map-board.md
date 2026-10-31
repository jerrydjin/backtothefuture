# Phase 14: map board; SF evidence blocked

Loaded maps use the same single white SVG board. Road polylines come from the
map loader's projected lat/lon geometry; the rush view now fits those roads
without including the unrelated dinner geometry in its bounds. Roads remain
thin grey, cars are small flat rectangles, granted emergency cars are red,
held cars amber, and pulled-over cars blue. Pull-over state is recorded in
frames, validated by replay, and survives export without new decisions.

`--demo --map` selects the boxed map replay (mock by default); `--demo --reason`
also uses the selected map, including its typed grant/denial twins. Without
--headless these modes serve the existing board. There is no new renderer,
traffic light, pedestrian, 3D view or model-driven motion.

The new integration test uses an explicitly synthetic imported map, proves
that the board's links match that map, checks both demo modes, and compares
saved archive bytes and exported HTML exactly. The browser check captured
one SVG, 39 cars, 4 red emergencies, 2 blue pulled-over cars, maximum road
width 2.6px and 0 JavaScript errors at fixture t=104. Its screenshot is
`docs/gates/phase-14-imported-fixture.png`. **This is not an SF screenshot.**

The requested SF boxed replay and screenshot are missing because phase 13's
real extract is blocked by the proxy's CONNECT HTTP 403. This phase remains
incomplete and a draft. No SF replay or driving metrics are fabricated.

Reproduce the available fixture checks (Playwright and Chromium installed):

```sh
cargo test --release --offline
cargo run --release -- import-osm tests/fixtures/import.osm --out /tmp/import-map.json
cargo run --release -- --demo --map /tmp/import-map.json --headless --policy mock --seed 1 --save-replay /tmp/import-board.json
cargo run --release -- --replay /tmp/import-board.json --export-replay /tmp/import-board.html
node scripts/check-map-board.cjs /tmp/import-board.html
```

Once the real extract is available (see maps/OSM.md), the required commands are:

```sh
muse --demo --map maps/sf-general.json --policy mock --seed 11 --save-replay recordings/sf-boxed-11.json
muse --demo --reason 'my passenger is having a seizure' --map maps/sf-general.json --policy mock --seed 11
muse --replay recordings/sf-boxed-11.json --export-replay /tmp/sf-boxed-11.html
```

Capture the actual SF board after replay/export, label it correctly and append
its measured gate result. Held-out seeds remain 11–20; no tuning follows failure.

| Gate | Target | Measured | Result |
|---|---|---|---|
| Sticky board | RED/RED/GREEN | board test and 9/9 sticky tests green | PASS |
| Loaded map, demo/reason/replay | identical map/motion/export | 1 integration test; exact archive bytes and HTML | PASS |
| Browser fixture | 1 SVG, red emergency, distinct pull-over | 1 SVG, 4 red, 2 blue, 0 JS errors | PASS |
| Real SF boxed saved replay + screenshot | actual SF, identical export/replay | 0 SF replays, 0 SF screenshots | BLOCKED |
| Cargo tests | no failures | 160 passed, 0 failed, 3 ignored | PASS |
| judge.rs/schema.rs; make_way.rs since phase 12 | unchanged | 3/3 | PASS |
