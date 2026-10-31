const {chromium}=require('playwright');
(async()=>{
 const server=require('http').createServer((req,res)=>{res.setHeader('Content-Type','text/html');res.end(require('fs').readFileSync(process.argv[2] || '/tmp/import-board.html'));});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage({viewport:{width:1280,height:800}});
 const errors=[];page.on('pageerror',e=>errors.push(String(e)));
 await page.goto('http://127.0.0.1:'+server.address().port+'/?t=104');
 await page.waitForTimeout(300);
 const measured=await page.evaluate(()=>({svgs:document.querySelectorAll('svg').length,
   emergency:document.querySelectorAll('rect[data-state="emergency"][fill="#d23b3b"]').length,
   pulledOver:document.querySelectorAll('rect[data-state="pull_over"][fill="#658bb0"]').length,
   cars:document.querySelectorAll('rect[data-car]').length,
   maxRoadWidth:Math.max(...Array.from(document.querySelectorAll('#network path[stroke="#e2e2e2"]')).map(p=>Number(p.getAttribute('stroke-width'))))}));
 if(errors.length||measured.svgs!==1||measured.emergency<1||measured.pulledOver<1||measured.maxRoadWidth>2.6)throw Error(JSON.stringify({errors,measured}));
 await page.screenshot({path:process.argv[3] || 'docs/gates/phase-14-imported-fixture.png'});
 require('fs').writeFileSync(process.argv[4] || 'docs/gates/phase-14-browser.json',JSON.stringify({fixture:'synthetic imported map; not SF',...measured,errors},null,2)+'\n');
 console.log(measured);await browser.close();server.close();
})().catch(e=>{console.error(e);process.exit(1)});
