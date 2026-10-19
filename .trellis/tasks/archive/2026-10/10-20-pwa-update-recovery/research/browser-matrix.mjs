const task=await taskSpace(27)
const page=task.page('p1')
const results=[]
for(const kind of ['update','install']) {
  for(const locale of ['zh-CN','en']) {
    for(const theme of ['light','dark']) {
      for(const [width,height] of [[320,568],[390,844],[448,900],[844,390]]) {
        await page.cdp('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:true})
        await page.evaluate(async ({kind,locale,theme})=>{
          document.documentElement.dataset.theme=theme
          window.fixture.i18n.global.locale.value=locale
          Object.assign(window.fixture.state,{enabled:true,isOnline:true,needRefresh:kind==='update',updateDismissed:false,updateError:kind==='update',updateFailure:'activation-timeout',updating:false,installError:false,installAvailable:kind==='install',iosInstallAvailable:false})
          await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
        },{kind,locale,theme})
        await page.waitForSelector(`#pwa-${kind}-dialog`,{state:'visible'})
        const geometry=await page.evaluate(()=>{
          const dialog=document.querySelector('[role=dialog]')
          const hint=dialog.querySelector('.pwa-install__hint')
          const description=dialog.querySelector('.pwa-install__description')
          return {
            width:dialog.getBoundingClientRect().width,height:dialog.getBoundingClientRect().height,
            dialogs:document.querySelectorAll('[role=dialog]').length,
            overflow:Math.max(document.body.scrollWidth,document.documentElement.scrollWidth)-innerWidth,
            dialogOverflow:dialog.scrollWidth-dialog.clientWidth,
            hintOverflow:hint.scrollWidth-hint.clientWidth,
            descriptionOverflow:description.scrollHeight-description.clientHeight,
            scrollable:dialog.scrollHeight>dialog.clientHeight,
            surface:getComputedStyle(dialog).backgroundColor,
          }
        })
        results.push({kind,locale,theme,viewportWidth:width,viewportHeight:height,...geometry})
        if(geometry.overflow>0||geometry.dialogOverflow>0||geometry.dialogs!==1||(kind==='update'&&(geometry.hintOverflow>0||geometry.descriptionOverflow>0))) throw new Error(JSON.stringify(results.at(-1)))
        if(kind==='update'&&locale==='en'&&theme==='dark'&&width===320) await page.screenshot({path:'/tmp/pwa-update-dark-en-320.png'})
        if(kind==='install'&&locale==='zh-CN'&&theme==='light'&&width===390) await page.screenshot({path:'/tmp/pwa-install-light-390.png'})
      }
    }
  }
}
const fs=await import('node:fs/promises')
await fs.writeFile('/Users/huangkunhuang/Public/程序工程目录/复合工程/rust-chain/.trellis/tasks/10-20-pwa-update-recovery/research/browser-results.json',JSON.stringify(results,null,2))
console.log({cases:results.length,failures:results.filter(x=>x.overflow||x.dialogOverflow),shortScreens:results.filter(x=>x.viewportHeight===390).length})
console.log(await page.snapshot())
