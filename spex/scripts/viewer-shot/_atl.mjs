import { chromium } from 'playwright';
const [base,out]=process.argv.slice(2);
const marks=[["atl-grandplace",250],["atl-giza",290],["atl-kolosseum",305],["atl-sydney",470]];
const b=await chromium.launch();
const p=await b.newPage({viewport:{width:1280,height:720}});
await p.goto(`${base}?duration=600&quality=low&mute=1`,{waitUntil:'networkidle'});
await p.waitForFunction(()=>!!window.__spexShow,null,{timeout:300000});
await p.evaluate(()=>window.__spexShow.begin());
await p.waitForTimeout(4000);
for(const [n,t] of marks){
  const i=await p.evaluate(async s=>{const x=window.__spexShow;x.setPlaying(false);x.seek(s);
    for(let k=0;k<3;k++)await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
    return {shot:x.activeShotId(),scenes:x.visibleScenes(),draws:x.drawCalls()};},t);
  await p.waitForTimeout(300);
  await p.screenshot({path:`${out}/${n}.png`,timeout:300000});
  console.log(n,'t='+t,i.shot,'scenes['+i.scenes+']',i.draws+' draws');
}
await b.close();
