# Final experiment freeze: getting to SF General

**On a real run to SF General through real traffic lights, the ambulance arrives 24.35 s sooner and no driver waits more than 128.75 s.** That is preempt-plus-clear against the siren-and-signal baseline, the mean over Station 7, Station 29 and Station 37, seeds 1–10. The 128.75 s is the worst extra wait among drivers who finished. Every run still has ordinary trips queued at 2,500 s, so those drivers sit outside that figure, and no seed passes the claim. Recorded collisions and overlaps are zero.
[Signal scores](gates/phase-13i-scores.jsonl)

Beside it, 13h on the smaller map with no signals and only Station 7: **the ambulance arrives 0.30 s sooner and no driver waits more than 6.00 s.** Gains there run from 2.25 s slower to 3.75 s faster. The ten trips gain 3.00 ambulance-seconds and cost 69.75 positive driver-seconds. Junction clearing passes the driver-cost bar on all ten seeds. Potrero broadcast still gains 13.18 s and leaves a driver 575.75 s later. Moving slot and v2 stay at 0 s.
[Whole-trip scores](gates/phase-13h-route-scores.jsonl)
[Potrero comparisons](gates/phase-13h-potrero-scores.jsonl)

The siren-and-signal baseline loses 15.12 s of extra time at each red, the mean of 74 recorded reds. The score file carries the cited comparison figure of 7 s of mean delay per signalised intersection, greens included. The two numbers sit side by side. No timing or demand was changed after the scores.

By station, preempt-plus-clear versus siren: Station 7 (2300 Folsom) averages 58.55 s, from 28.00 s to 101.25 s, and every seed fails the cost bar (95th percentile 83.25 s; worst finished driver 128.75 s). Station 29 (299 Vermont) gains 14.50 s on every seed; the 95th percentile is 5.25 s and one driver loses 90.00 s. Station 37 (798 Wisconsin) gains 0.00 s. Its only retained light is already green. The finished-driver bar passes there, and the run is invalid because other trips do not finish. With the lights present, junction clearing alone averages 8.04 s and the worst seed is 28.25 s slower. Preemption alone averages 19.69 s. Waiting out the red, with the siren procedure off, is 32.98 s slower than the siren baseline on average.

The best seed is Station 7, seed 5: 669.50 s versus 770.75 s, a gain of 101.25 s. 19th/Folsom and 18th/Folsom stay red in both arms (11.62 s and 6.22 s). The ambulance is at the first light one step after departure, so the call cannot finish yellow and all-red before it arrives. 17th/Folsom opens the gap to 24.00 s (the siren arm loses 23.58 s there). 17th/Harrison takes it to 40.00 s (siren loses 14.39 s). 17th/Bryant takes it to 88.00 s (siren loses 46.24 s). Mariposa/Potrero takes it to 100.75 s (siren loses 10.71 s). 23rd/Potrero is green in both arms and the lead holds at 101.25 s. The worst seed is Station 37, seed 1: both arms arrive at 413.75 s. The only signal, 23rd/Potrero, is entered at 397.50 s in both arms and occupied for 2.00 s. Every junction on that route shows 0 s.

Eight of the nine signals run longer than the cited 60 s cycle, up to 92.28 s, because pedestrian clearance and minimum green on four or five conflicting approaches do not fit in 60 s. 23rd/Potrero stays at 60 s with one retained approach. A short green cannot discharge the cited 1,250 vehicles per hour. About half the trips are still queued at 2,500 s. That queue is the result.

Hampshire: diverting traffic without knowing where it rejoins moved the jam into the ambulance's path (−51.75 s, 13h).

These are simulations. The driving model is uncalibrated. The streets are OSM geometry. Demand and the signal timing are the cited San Francisco standard (standard, not measured). The preemption message is ideal. The result stays frozen, including the zero gains, the failed cost bar and the drivers who never finish.
