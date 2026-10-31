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
  const incidentRecords = town.incidents || [];
  if(incidentRecords.length){
    assert.deepEqual(incidentRecords[0].definition, JSON.parse(fs.readFileSync('scenarios/sf-potrero-incident.json')));
  }
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
    let blockedLaneSamplesChecked=0;
    for(const record of incidentRecords){
      const i=record.definition;
      for(const frame of town.frames.filter(f=>f.t>=Math.max(startS,i.start_s+1)&&f.t<i.start_s+i.duration_s)){
        for(const car of frame.cars){
          let start=0;
          for(const link of town.routes[car[1]]){
            if(link===record.link){
              const rear=car[2]-board.car_length_m-start,front=car[2]-start;
              if(front>=0&&rear<=town.links[link].len){
                blockedLaneSamplesChecked++;
                assert.ok(!(rear<i.position_m+i.length_m-.05 && front>i.position_m+.05),'body overlaps the active blocked footprint');
              }
            }
            start+=town.links[link].len;
          }
        }
      }
    }
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
        received: [...document.querySelectorAll('[data-role="rush-received"]')].map(n => ({car:Number(n.dataset.car),beneficiary:Number(n.dataset.beneficiary),at:Number(n.dataset.deliveredAt)})),
        fronts: [...document.querySelectorAll('[data-role="rush-pulse"]')].map(n => ({ beneficiary: Number(n.dataset.beneficiary), head: Number(n.dataset.headM), car: Number(n.dataset.carM), width: Number(n.getAttribute('stroke-width')), evidence: n.dataset.evidence })),
        maxRoadWidth: Math.max(...[...document.querySelectorAll('#network path[stroke="#e2e2e2"]')].map(n => Number(n.getAttribute('stroke-width')))),
        incidents: [...document.querySelectorAll('[data-role="lane-blockage"]')].map(n => ({id:n.dataset.incident,link:Number(n.dataset.link),position:Number(n.dataset.positionM),length:Number(n.dataset.lengthM),start:Number(n.dataset.startS),end:Number(n.dataset.endS)})),
        svgs: document.querySelectorAll('svg').length
      }));
      assert.equal(measured.svgs, 1); assert.match(measured.attribution, /OpenStreetMap.*ODbL/);
      assert.ok(measured.maxRoadWidth <= 2.6);
      const activeIncidents = incidentRecords.filter(r => r.definition.start_s <= t && t < r.definition.start_s + r.definition.duration_s);
      assert.deepEqual(measured.incidents.map(i=>i.id).sort(),activeIncidents.map(r=>r.definition.id).sort());
      for(const visible of measured.incidents){
        const declared=activeIncidents.find(r=>r.definition.id===visible.id);
        assert.equal(visible.link,declared.link);assert.equal(visible.position,declared.definition.position_m);assert.equal(visible.length,declared.definition.length_m);
      }
      for (const reaction of measured.reacting) assert.ok(messages.events.some(h => h.recipient_car === reaction.car && h.action === (reaction.state === 'pull_over' ? 'lane_change' : reaction.state) && h.at_s <= t));
      for (const receipt of measured.received) {
        assert.ok(messages.events.some(h => h.action === 'relay' && h.recipient_car === receipt.car && h.beneficiary === receipt.beneficiary && h.at_s === receipt.at && h.at_s <= t && t < h.at_s + 1), 'receipt cue before recorded delivery');
      }
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
    for(const record of incidentRecords){
      const i=record.definition;
      for(const t of [Math.max(0,i.start_s-.001),i.start_s,i.start_s+i.duration_s-.001,i.start_s+i.duration_s])await inspect(t);
    }
    const actions = messages.events.filter(h => h.at_s >= startS && (!incidentRecords.length || incidentRecords.some(r=>r.definition.start_s<=h.at_s&&h.at_s<r.definition.start_s+r.definition.duration_s)) && (['hold', 'clear', 'lane_change'].includes(h.action) || (h.action === 'relay' && h.recipient_car != null)));
    const times = [...new Set(actions.slice(0, 4).flatMap(h => [Math.max(0, h.at_s - .01), h.at_s, Math.ceil(h.at_s)]))];
    for (const t of times) await inspect(t);
    let screenshot;
    for (const t of [...new Set(actions.flatMap(h => [h.at_s + .01, Math.ceil(h.at_s)]))].concat([startS + 45, startS + 60, startS + 90, startS + 120]).slice(0, 35)) {
      const measured = await inspect(t);
      if (measured.emergency >= 1 && (incidentRecords.length ? (measured.incidents.length && measured.reacting.length) : (measured.reacting.length || measured.received.length)) && measured.fronts.some(f => f.head - f.car > 100)) {
        await page.screenshot({ path: pngPath }); screenshot = measured; break;
      }
    }
    assert.ok(screenshot, 'no real SF message front ahead of red car');
    assert.equal(errors.length, 0);
    const report = { blockedLaneSamplesChecked, drivingManeuvers: messages.events.filter(h => h.action !== 'relay').length, receiptCuesChecked: samples.reduce((n,s)=>n+s.received.length,0), source: 'maps/sf-zsfg.osm', score: rush.score, checkedStates, screenshot, samples, errors };
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n');
    console.log(JSON.stringify({ checkedStates, screenshotTime: screenshot.time, fronts: screenshot.fronts, errors }));
  } finally { await browser.close(); server.close(); }
})().catch(e => { console.error(e); process.exit(1); });
