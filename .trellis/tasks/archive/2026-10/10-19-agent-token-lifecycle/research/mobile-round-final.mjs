const task=await taskSpace(22);const p=task.page('p1');
for(const width of [320,390,448])for(const theme of ['light','dark'])for(const locale of ['zh-CN','en']){
 await p.cdp('Emulation.setDeviceMetricsOverride',{width,height:844,deviceScaleFactor:1,mobile:true});
 await p.evaluate(async({theme,locale})=>{(await import('/src/stores/theme.ts')).useThemeStore().setTheme(theme);(await import('/src/i18n/index.ts')).setAppLocale(locale);await new Promise(requestAnimationFrame);},{theme,locale});
 const result=await p.evaluate(()=>({overflow:document.documentElement.scrollWidth-innerWidth,chart:document.querySelector('.mobile-market-chart')?.getAttribute('data-chart-state'),narrow:[...document.querySelectorAll('.token-trading > form button,.token-holding button,.token-trading > header button')].filter(e=>e.getBoundingClientRect().height<44).length}));
 if(result.overflow||result.narrow)throw new Error(JSON.stringify({width,theme,locale,result}));console.log({width,theme,locale,...result});
 if(width===390&&locale==='en'){
  await p.evaluate(()=>{for(const e of document.querySelectorAll('*'))if(e.scrollTop)e.scrollTop=0});
  console.log(await p.screenshot({path:`/tmp/token-mobile-${theme}-en.png`,fullPage:true}));
 }
}
await p.cdp('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});
await p.evaluate(async()=>{(await import('/src/i18n/index.ts')).setAppLocale('zh-CN');window.__tokenQA.holdingsFail=true;document.querySelector('.token-trading > header button').click();});
await p.waitForSelector('.token-trading > [role=alert]',{timeout:5000});
console.log('error retains holding',await p.evaluate(()=>!!document.querySelector('.token-holding')));
await p.evaluate(()=>{window.__tokenQA.holdingsFail=false;document.querySelector('.token-trading > header button').click()});
await p.waitForFunction(()=>!document.querySelector('.token-trading > [role=alert]'),undefined,{timeout:5000});
await p.evaluate(async()=>{const {default:router}=await import('/src/router/index.ts');window.__tokenQA.project.lifecycle_status='subscription';await router.push({name:'new-coins'});await router.push({name:'new-coin-detail',params:{symbol:'NOVA'}})});
await p.waitForSelector('.token-subscribe input',{timeout:5000});
await p.evaluate(()=>document.querySelector('.token-subscribe input').scrollIntoView({block:'center'}));
await p.fill('.token-subscribe input','5000',{timeout:5000});await p.click('loc=role:button[name="核对申购"]',{timeout:5000});
console.log('subscription review',await p.evaluate(()=>document.querySelector('.token-subscribe')?.textContent));
await p.click('.token-subscribe button:text-is("确认成交")',{timeout:5000});
await p.waitForSelector('.token-subscribe [role=status]',{timeout:5000});console.log('subscription',await p.evaluate(()=>document.querySelector('.token-subscribe [role=status]')?.textContent));
await p.evaluate(async()=>{const {default:router}=await import('/src/router/index.ts');window.__tokenQA.project.lifecycle_status='listed';await router.push({name:'new-coins'});await router.push({name:'new-coin-detail',params:{symbol:'NOVA'}})});
await p.waitForSelector('loc=role:button[name="前往现货交易"]',{timeout:5000});console.log('listed purchase form absent',await p.evaluate(()=>!document.querySelector('.token-trading > form')));
await p.evaluate(()=>[...document.querySelectorAll('button')].find(e=>e.textContent==='前往现货交易').scrollIntoView({block:'center'}));
await p.click('loc=role:button[name="前往现货交易"]',{timeout:5000});await p.waitForURL('**/trade/NOVAUSDT',{timeout:5000});console.log('spot',await p.url());
await p.evaluate(async()=>{const {default:router}=await import('/src/router/index.ts');window.__tokenQA.project.issuance_policy=null;window.__tokenQA.project.lifecycle_status='subscription';await router.push({name:'new-coin-detail',params:{symbol:'NOVA'}})});
await p.waitForSelector('.new-coin-detail',{timeout:5000});console.log('v1',await p.evaluate(()=>({v2Form:!!document.querySelector('.token-subscribe'),overflow:document.documentElement.scrollWidth-innerWidth})));
console.log(await p.screenshot({path:'/tmp/token-mobile-v1.png',fullPage:true}));
