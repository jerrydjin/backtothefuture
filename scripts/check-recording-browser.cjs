const { chromium } = require('playwright');
const { spawn } = require('child_process');
const fs = require('fs');
const assert = require('assert');
(async () => {
  const child = spawn('python3', ['scripts/play-recording.py','--legacy','--no-open','--binary','target/debug/muse']);
  let stdout = '';
  const url = await new Promise((resolve,reject) => {
    child.stdout.on('data', b => { stdout += b; const m=stdout.match(/recording: (http:\/\/[^\s]+)/); if(m)resolve(m[1]); });
    child.on('exit', () => reject(new Error('player failed before readiness')));
  });
  const browser = await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
  try {
    const page = await browser.newPage({viewport:{width:1440,height:1000}});
    await page.clock.install();
    const errors=[], requests=[];
    page.on('pageerror', e => errors.push(e.message));
    page.on('request', r => requests.push(r.url()));
    await page.route('**/*', route => new URL(route.request().url()).hostname === '127.0.0.1' ? route.continue() : route.abort());
    const base = new URL(url).origin;
    const demo = JSON.parse(fs.readFileSync('recordings/demo.json'));
    for (const [event, t] of [[0,27],[1,61],[2,96]]) {
      await page.goto(base+'/demo.html?replay=1&t='+t);
      assert.equal(await page.locator('svg').count(),1);
      assert.equal(await page.locator('#network').getAttribute('data-demo-phase'),'dialogue');
      for (const line of demo.board.town.exchanges[event].lines) {
        assert.equal(await page.locator('[data-speaker="'+line.id+'"]').textContent(),line.say);
      }
    }
    await page.goto(base+'/demo.html?replay=1');
    await page.clock.fastForward(102000);
    await page.waitForURL('**/grant.html?replay=1');
    await page.clock.fastForward(35100);
    await page.waitForURL('**/deny.html?replay=1');
    await page.clock.fastForward(14100);
    assert.equal(await page.locator('#network').getAttribute('data-recording-complete'),'true');
    await page.clock.resume();
    for (const clip of ['grant','deny']) {
      const saved=JSON.parse(fs.readFileSync('recordings/'+clip+'.json'));
      await page.goto(base+'/'+clip+'.html?replay=1&t=205');
      assert.equal(await page.locator('svg').count(),1);
      assert.equal(await page.locator('input:visible').count(),0);
      assert((await page.locator('[data-role="reason-explanation"]').textContent()).includes(saved.board.reason.decision.explanation));
      for (const line of saved.board.reason.decision.lines) {
        assert.equal(await page.locator('[data-role="reason-line"][data-car="'+line.id+'"]').textContent(),line.say);
      }
      await page.screenshot({path:'/tmp/phase-c-'+clip+'.png'});
    }
    assert.deepEqual(errors,[]);
    assert(!requests.some(x=>/\/(negotiate|result|demo\/state|reason)(?:\?|$)/.test(x)));
    console.log(JSON.stringify({svg:1,visible_inputs:0,exact_saved_lines:true,playlist_advanced:true,denial_freezes:true,model_requests:0,page_errors:errors}));
  } finally { await browser.close(); child.kill('SIGTERM'); }
})().catch(e=>{console.error(e.message);process.exitCode=1;});
