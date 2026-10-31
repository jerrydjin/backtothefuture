// Headless real-map evidence and delivery-time regression. Requires Playwright.
const fs = require('fs');
const assert = require('assert/strict');
const { chromium } = require('playwright');
(async () => {
  const [htmlPath, replayPath, messagesPath, pngPath, reportPath, startArg] = process.argv.slice(2);
  const startS = Number(startArg || 0);
  const archive = JSON.parse(fs.readFileSync(replayPath));
  const board = archive.board, rush = board.rush, town = board.town;
  const messages = JSON.parse(fs.readFileSync(messagesPath));
  assert.deepEqual(rush.message_log, messages);
  assert.deepEqual(rush.messages, messages.events);
  assert.match(town.attribution, /OpenStreetMap/);
  const server = require('http').createServer((req, res) => {
    res.setHeader('Content-Type', 'text/html'); res.end(fs.readFileSync(htmlPath));
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = []; page.on('pageerror', e => errors.push(String(e)));
    const samples = [];
    // Every first reaction must have a delivery by that frame, including
    // ordinary holds and pulls close to the emergency, not just far cues.
    let checkedStates = 0;
    for (const frame of town.frames) for (const [car, beneficiary, state] of frame.make_way || []) {
      const action = state === 'pull_over' ? 'lane_change' : state;
      const delivery = messages.events.find(h => h.recipient_car === car && h.beneficiary === beneficiary && h.action === action && h.at_s <= frame.t);
      assert.ok(delivery, `reaction before delivery: car ${car}, ${state}, t=${frame.t}`);
      checkedStates++;
    }
    async function inspect(t) {
      await page.goto(`http://127.0.0.1:${server.address().port}/?t=${t}`);
      await page.waitForFunction(t => Math.abs(Number(document.querySelector('#network').dataset.time) - t) < .002, t);
      const measured = await page.evaluate(() => ({
        time: Number(document.querySelector('#network').dataset.time),
        attribution: document.querySelector('[data-role="map-attribution"]')?.textContent,
        emergency: document.querySelectorAll('rect[data-state="emergency"][fill="#d23b3b"]').length,
        reacting: [...document.querySelectorAll('rect[data-car]')].filter(n => ['hold', 'pull_over', 'clear'].includes(n.dataset.state)).map(n => ({ car: Number(n.dataset.car), state: n.dataset.state })),
        fronts: [...document.querySelectorAll('[data-role="rush-pulse"]')].map(n => ({ beneficiary: Number(n.dataset.beneficiary), head: Number(n.dataset.headM), car: Number(n.dataset.carM), width: Number(n.getAttribute('stroke-width')), evidence: n.dataset.evidence })),
        maxRoadWidth: Math.max(...[...document.querySelectorAll('#network path[stroke="#e2e2e2"]')].map(n => Number(n.getAttribute('stroke-width')))),
        svgs: document.querySelectorAll('svg').length
      }));
      assert.equal(measured.svgs, 1); assert.match(measured.attribution, /OpenStreetMap.*ODbL/);
      assert.ok(measured.maxRoadWidth <= 2.6);
      for (const reaction of measured.reacting) assert.ok(messages.events.some(h => h.recipient_car === reaction.car && h.action === (reaction.state === 'pull_over' ? 'lane_change' : reaction.state) && h.at_s <= t));
      for (const front of measured.fronts) {
        assert.equal(front.width, 1); assert.equal(front.evidence, 'save-messages'); assert.ok(front.head > front.car);
        const request = messages.requests.find(r => r.issuer === front.beneficiary && r.issued_at <= t && t < r.expires_at);
        assert.ok(request);
        const hops = messages.events.filter(h => h.beneficiary === front.beneficiary && h.at_s >= request.issued_at && h.at_s < request.expires_at).sort((a, b) => a.at_s - b.at_s || a.route_m - b.route_m);
        const origin = hops.find(h => h.action === 'relay') || hops[0];
        const head = Math.max(origin.route_m - origin.ahead_m, ...hops.filter(h => h.at_s <= t).map(h => h.route_m));
        assert.ok(front.head <= head + 1e-6, 'front runs ahead of delivered data');
        assert.ok(Math.abs(front.head - head) < 1e-6, 'front differs from saved timestamps');
        for (const h of hops.filter(h => h.at_s <= t)) assert.ok(front.head + 1e-6 >= h.route_m, 'reaction delivered beyond the visible message front');
      }
      samples.push(measured); return measured;
    }
    const actions = messages.events.filter(h => h.at_s >= startS && ['hold', 'clear', 'lane_change'].includes(h.action));
    const times = [...new Set(actions.slice(0, 4).flatMap(h => [Math.max(0, h.at_s - .01), h.at_s, Math.ceil(h.at_s)]))];
    for (const t of times) await inspect(t);
    let screenshot;
    for (const t of [...new Set(actions.map(h => Math.ceil(h.at_s)))].concat([startS + 45, startS + 60, startS + 90, startS + 120]).slice(0, 35)) {
      const measured = await inspect(t);
      if (measured.emergency >= 1 && measured.reacting.length && measured.fronts.some(f => f.head - f.car > 100)) {
        await page.screenshot({ path: pngPath }); screenshot = measured; break;
      }
    }
    assert.ok(screenshot, 'no real SF message front ahead of red car');
    assert.equal(errors.length, 0);
    const report = { source: 'maps/sf-zsfg.osm', score: rush.score, checkedStates, screenshot, samples, errors };
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n');
    console.log(JSON.stringify({ checkedStates, screenshotTime: screenshot.time, fronts: screenshot.fronts, errors }));
  } finally { await browser.close(); server.close(); }
})().catch(e => { console.error(e); process.exit(1); });
