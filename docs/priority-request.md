# Priority request (JERRY-143)

One symbolic priority request on the existing town board. Cars ahead relay it
at successive junctions, negotiate a clear path, and execute the returned
agreements with the existing collision and gap checks. The exchange uses the
same short on-map lines and late-reply backup as PR #11.

Acceptance: at least two negotiations along the requested route; the same car
gets home measurably sooner than in a paired no-request run with the same seed,
route, demand, and physics; no lane overlaps or crossing-path collisions.
Sticky RED/RED/GREEN time_home=69 and the protected judge/schema stay unchanged.
A scripted fallback must complete this demonstration without credentials.

Implementation and measured results follow in this branch. This PR is stacked
on cursor/town-merge-talk-96f5; it must not be merged by the agent.
