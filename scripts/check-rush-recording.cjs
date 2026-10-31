const {chromium}=require('playwright');
const {spawn}=require('child_process');
const fs=require('fs'),assert=require('assert');
let browser;
const child=spawn('python3',['scripts/play-recording.py','--no-open','--binary','target/debug/muse']);
(async()=>{
  let output='';
  const url=await new Promise((resolve,reject)=>{child.stdout.on('data',b=>{output+=b;const m=output.match(/recording: (http:\/\/[^\s]+)/);if(m)resolve(m[1]);});child.on('exit',()=>reject(new Error('player exited before readiness')));});
  browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[],requests=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(r.url()));
  await page.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
  const base=new URL(url).origin;
  const far=JSON.parse(fs.readFileSync('docs/gates/phase-5-browser.json')).far;
  const counts={};
  for(const clip of ['rush-hour','rush-grant','rush-liar','rush-deny']){
    const saved=JSON.parse(fs.readFileSync('recordings/'+clip+'.json'));
    assert(saved.board.rush.decisions.every(d=>d.source==='model'));
    counts[clip]=saved.board.rush.decisions.length;
    const t=clip==='rush-hour'?far.at_s:saved.board.rush.typed.decision.at_s+1;
    await page.goto(base+'/'+clip+'.html?replay=1&t='+t);
    assert.equal(await page.locator('svg').count(),1);assert.equal(await page.locator('input:visible').count(),0);
    const board=JSON.parse(await page.locator('#board-data').textContent());assert.deepEqual(board,saved.board);
    assert.equal(board.rush.score.collisions,0);
    if(clip==='rush-hour'){
      assert(await page.locator('[data-role="rush-make-way"]').count()>0);
      assert((await page.locator('[data-role="rush-score"]').textContent()).includes('0 liars caught'));
      await page.screenshot({path:'docs/gates/phase-5-offline.png'});
    }else{
      const d=saved.board.rush.typed.decision;
      assert.deepEqual(await page.locator('[data-role="rush-line"]').allTextContents(),[d.requester,d.other]);
      if(clip==='rush-liar')assert((await page.locator('[data-role="rush-score"]').textContent()).includes('1 liars caught'));
    }
  }
  await page.clock.install();
  await page.goto(url);
  await page.clock.fastForward(120100);await page.waitForURL('**/rush-grant.html?replay=1');
  await page.clock.fastForward(40100);await page.waitForURL('**/rush-liar.html?replay=1');
  await page.clock.fastForward(40100);await page.waitForURL('**/rush-deny.html?replay=1');
  await page.clock.fastForward(40100);
  await page.clock.fastForward(300);
  await page.waitForFunction(()=>document.querySelector('#network').dataset.recordingComplete==='true');
  assert.equal(await page.locator('#network').getAttribute('data-recording-complete'),'true');
  assert.deepEqual(errors,[]);
  assert(!requests.some(x=>new URL(x).hostname!=='127.0.0.1'||/\/(negotiate|result|reason|rush-reason|demo\/state)(?:\?|$)/.test(x)));
  const report={archives:counts,svg:1,visible_inputs:0,exact_saved_board:true,playlist_complete:true,model_requests:0,external_requests:0,page_errors:errors};
  fs.writeFileSync('docs/gates/phase-5-offline.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report));
})().catch(e=>{console.error(e.stack);process.exitCode=1;}).finally(async()=>{if(browser)await browser.close();child.kill('SIGTERM');});
