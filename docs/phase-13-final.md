# Final experiment freeze: getting to SF General

**On a real run to SF General, the ambulance arrives 0.30 s sooner and no
driver waits more than 6.00 s.** That is the simulated result versus the
preregistered local siren, across every qualifying station and seed. The map
contains one qualifying station, Station 7. Junction clearing passes the
unchanged driver-cost bar on all ten seeds, with zero recorded collisions.
But gains range from **2.25 s slower to 3.75 s faster**; the small mean is not
evidence of a substantial whole-trip improvement.
[Whole-trip scores](gates/phase-13h-route-scores.jsonl)

The ten trips gain **3.00 ambulance-seconds** in total and cost **69.75 positive
driver-seconds** of arrival delay. There are **38 driver-run active holds**,
totaling **239.50 s**; held drivers keep approaching rather than standing still.
The ambulance itself spends **5.75 s below 1 m/s** in junction approach zones
under siren and **9.00 s** with clearing. Early holds sometimes release cross
traffic into the ambulance's later arrival window. In the best seed, a later
hold moves a conflicting driver behind the ambulance and saves time. The
predictor can help at one crossing and hurt at another.
[Whole-trip scores and hold/junction evidence](gates/phase-13h-route-scores.jsonl)

Potrero's jammed merge offers scarce space, so changing precedence mostly
changes who waits. Broadcast still gains **13.18 s** on average but leaves
another driver **575.75 s** later: every seed fails the cost bar. Moving slot
13e and v2 13f remain **0 s**. The two cited-gap variants gain **2.1275 s** but
cost one driver up to **531.00 s**; only seed 1 passes. Moving a predecessor
aside can leave it stranded behind a refilled gap. Those costs are not free
capacity. The old scores remain intact.
[New Potrero comparisons versus siren](gates/phase-13h-potrero-scores.jsonl)

The real Hampshire bypass is used by **952 ordinary driver-runs**, and by
**no ambulance**. It moves the queue to the downstream rejoin: the ambulance
arrives **51.75 s later** on average, and the worst driver loses **62.50 s**.
Only seeds **4 and 10** pass the cost bar. The new upstream admission setup
also records body-overlap observations in seeds **3, 6 and 10**, in every arm
including controls; seed 10 cannot support a safety claim. These are retained
failures. The map-based dead-end rule rejects **zero** proposals: reachability
alone does not prevent downstream congestion or unsafe admission.
[Bypass scores, validity and collisions](gates/phase-13h-potrero-scores.jsonl)

These are real-map simulations, not field travel times. The host has connector
claims and turn-speed limits, **no traffic signals**, and already generous
emergency priority. The conditional siren benchmark is not calibrated to local
hearing or sight lines; cross-street demand is assumed. OSM geometry, station
snapping, source admission and the uncalibrated driving model limit the claim.
This phase does not establish that junction clearing or diversion is ready
for real emergency traffic. The result stays frozen, including zero gains,
negative gains and safety failures.
