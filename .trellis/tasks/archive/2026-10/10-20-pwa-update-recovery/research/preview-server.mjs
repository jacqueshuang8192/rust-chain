import { createServer } from '../../../../mobile/node_modules/vite/dist/node/index.js'
let version = 1
const html = `<!doctype html><html lang="zh-CN" data-theme="light"><head><meta name="viewport" content="width=device-width, initial-scale=1"><title>PWA update verification</title></head><body style="margin:0"><div id="fixture"></div><script type="module">
import {createApp, h, reactive, toRaw} from 'vue';
import {createI18n} from 'vue-i18n';
import {createRouter, createWebHashHistory} from 'vue-router';
import PwaStatus from '/src/components/PwaStatus.vue';
import * as pwa from '/src/pwa/index.ts';
import zh from '/src/i18n/messages/zh-CN.ts';
import en from '/src/i18n/messages/en.ts';
import '/src/styles/base.css';
const i18n = createI18n({legacy:false,locale:'zh-CN',messages:{'zh-CN':zh,en}});
const router=createRouter({history:createWebHashHistory(),routes:[{path:'/',name:'home',component:{render:()=>null}},{path:'/trade',name:'trade',component:{render:()=>null}}]});
const app=createApp({render:()=>h('main',{style:'max-width:448px;margin:auto;padding:24px;min-height:150vh;background:#eff8f3;color:#102018'},[h('h1','Hippo'),h('p','Local PWA verification'),h('button',{id:'trigger',onClick:()=>navigator.serviceWorker.getRegistration().then(r=>r.update())},'Check version'),h(PwaStatus)])});
app.use(router);app.use(i18n);
await router.isReady();
const reloaded=await pwa.initializePwa();
if(!reloaded) app.mount('#fixture');
window.fixture={state:reactive(toRaw(pwa.pwaState)),i18n,router,pwa};
</script></body></html>`
const server = await createServer({
  root: process.cwd(), configFile: 'vite.config.ts', mode: 'pwa',
  server: { host: '127.0.0.1', port: 5187, strictPort: true },
  plugins: [{name:'local-pwa-verification', configureServer(vite) {
    vite.middlewares.use(async (req,res,next)=>{
      if(req.url === '/__pwa-version' && req.method === 'POST') { version++; res.end(String(version)); return }
      if(req.url === '/sw.js') {
        res.setHeader('Content-Type','application/javascript');res.setHeader('Cache-Control','no-store');
        res.end(`const version=${version};self.addEventListener('message',event=>{if(event.data?.type==='SKIP_WAITING')self.skipWaiting()});`);return
      }
      if(req.url === '/__pwa-fixture') {res.setHeader('Content-Type','text/html');res.setHeader('Cache-Control','no-store');res.end(await vite.transformIndexHtml('/__pwa-fixture',html));return}
      next()
    })
  }}],
})
await server.listen()
console.log('Local PWA fixture ready at http://127.0.0.1:5187/__pwa-fixture')
