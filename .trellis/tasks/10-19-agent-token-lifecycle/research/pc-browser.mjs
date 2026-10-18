const task=await taskSpace(22);const fixture=await task.page('p1').evaluate(()=>({project:{...window.__tokenQA.project,issuance_policy:window.__tokenQA.policy,lifecycle_status:'purchase'},holding:window.__tokenQA.holding}));
const p=await task.newPage();console.log('pcPage',p.label);await p.goto('http://localhost:5173');
await p.evaluate(async fixture=>{
 const {default:request}=await import('/src/api/request.ts');const {default:router}=await import('/src/router/index.ts');const {useUserStore}=await import('/src/stores/user.ts');
 window.__pcStorageBackup=['user','token','refresh_token','hippo_pc_session_scope'].map(key=>[key,localStorage.getItem(key)]);
 window.__tokenPC={...fixture,writes:[],reads:[],quotes:{}};request.instance.defaults.adapter=async config=>{
 const f=window.__tokenPC;const path=new URL(config.url,location.origin).pathname;const body=typeof config.data==='string'?JSON.parse(config.data):config.data;let data={};if(config.method!=='get')f.writes.push({path,body});else f.reads.push(path);
 if(path.endsWith('/new-coins/NOVA'))data=f.project;
 else if(path.endsWith('/new-coins'))data={projects:[f.project]};
 else if(path.endsWith('/new-coins/holdings'))data={holdings:[f.holding],next_before_id:null,checked_at:Date.now()};
 else if(path.endsWith('/new-coins/NOVA/quotes')){const sell=body.side==='sell';data={id:body.idempotency_key,project_id:700,side:body.side,holding_id:sell?7001:null,quantity:sell?'2500':body.quantity,price:'0.4',gross:sell?'1000':'4',cost:sell?'500':'0',fee:sell?'100':'0',net:sell?'900':'4',observed_at:Date.now(),expires_at:Date.now()+15000,status:'quoted',receipt_json:null};f.quotes[data.id]=data;}
 else if(path.includes('/new-coins/quotes/')&&path.endsWith('/confirm')){const q=f.quotes[path.split('/').at(-2)];data={quote_id:q.id,project_id:700,side:q.side,quantity:q.quantity,price:q.price,gross:q.gross,fee:q.fee,net:q.net,holding_id:7001,executed_at:Date.now()};}
 else if(path.endsWith('/klines'))data=Array.from({length:60},(_,i)=>({symbol:'NOVAUSDT',open_time:Date.now()-60000*(60-i),open:'0.3',high:'0.41',low:'0.28',close:String((300+i)/1000),volume:'100',provider:'strategy',source:'default'}));
 else if(path.endsWith('/markets'))data={markets:[]};else if(path.includes('/wallet'))data={accounts:[]};
 return {data,status:200,statusText:'OK',headers:{},config};
 };
 useUserStore().setAuthSession({token:`eyJhbGciOiJub25lIn0.${btoa(JSON.stringify({sub:'user:token-local-qa'}))}.qa`,refreshToken:'pc-local-qa'});await router.push('/launchpad/project/NOVA');
},fixture);
await p.waitForSelector('.token-holding',{timeout:10000});console.log(await p.snapshot({scope:'full_page'}));
