The SF experiment now starts on the real southbound Potrero arterial approach to ZSFG, with the route and demand committed before scores (085057d and 0c4a1bb). SF Public Works’ Potrero Corridor presentation, 30 July 2013, slide 11, publishes 1,000–1,500 vehicles/hour per direction. Use its fixed midpoint, 1,250 vehicles/hour southbound (2.88 s headway), with ten emergency probes replacing ordinary trips during a 900 s run. This historical rate is directional, not per lane.

Source: https://sfpublicworks.org/sites/default/files/3526-13_0730_Potrero%20Stscp%20Presentation.pdf#page=11

Stack on codex/phase-13b-real-sf (#35). The map carries the cited profile and explicit Potrero edge path without geometry or driving-policy changes. Native town behavior is preserved. Reporting retains the same FCFS/local/broadcast runs and compares FCFS emergencies with solo trips.

Results, seeds 1–10: all 3,125 trips complete with zero collisions. Median FCFS delay by seed is 6.0–6.25 seconds. Gain vs FCFS and vs local is exactly 0 seconds, and others mean/p95 is 0 throughout. All probes are pull-over-able; single-lane has zero samples and null metrics. Demand was not increased after seeing the results.

The mid-run seed-1 screenshot and replay show a 1 px front driven only by delivered message events. Drivers receive messages and continue; no physical driving maneuvers occurred. Receipt cues are explicitly labeled so they do not imply a pull-over or benefit.

Validation: 167 offline tests pass, 0 fail, 3 existing live tests ignored; 40/40 town score comparisons byte-identical to main 255baa9, exact archive round-trip, timestamp/receipt browser checks and zero JavaScript errors. judge.rs/schema.rs and town.json unchanged; make_way.rs unchanged from Phase 13b. No live runs; Michael runs live gates. Full per-seed/type data and limitations: docs/phase-13c-sf-demand.md.
