const task = await taskSpace(22); const page = task.page('p1');
await page.cdp('Emulation.setDeviceMetricsOverride', { width:390,height:844,deviceScaleFactor:1,mobile:true });
await page.evaluate(async () => {
  const { client, persistAuthTokens } = await import('/src/api/client.ts');
  const { default: router } = await import('/src/router/index.ts');
  const now=Date.now();
  const policy={version:2,timezone:'Asia/Hong_Kong',purchase_initial_price:'0.3',listing_reference_price:'9.99',subscription:{at:now-86400000*4,automatic:true},distribution:{at:now-86400000*3,automatic:false},purchase:{at:now-86400000,automatic:true},listing:{at:now+86400000*7,automatic:false},lock:{kind:'on_purchase'},fee:{kind:'profit',rate:'0.2'}};
  const project={id:700,asset_id:701,symbol:'NOVA',name:'NOVA test issuance',logo_url:null,lifecycle_status:'purchase',total_supply:'20000',issue_price:'0.2',quote_asset_id:702,quote_asset_symbol:'USDT',quote_asset_logo_url:null,reserved_supply:'0',allocated_supply:'6500',remaining_supply:'13500',listed_at:policy.listing.at,unlock_type:'immediate_on_listing',fixed_unlock_at:null,relative_unlock_seconds:null,unlock_fee_enabled:false,unlock_fee_rate:null,unlock_fee_basis:null,unlock_fee_asset:null,post_listing_purchase_enabled:false,post_listing_pair_id:703,post_listing_pair_symbol:'NOVAUSDT',status:'active',issuance_policy:policy};
  const holding={holding:{id:7001,project_id:700,quantity:'2500',cost:'500',status:'held',acquired_at:now-86400000*4,sold_at:null,policy_json:policy},symbol:'NOVA',quote_symbol:'USDT',pair_id:703,pair_symbol:'NOVAUSDT',lifecycle_status:'purchase',eligible:true,unlock_at:now-86400000,market_price:'0.4',market_value:'1000',profit:'500'};
  window.__tokenQA={project,policy,holding,writes:[],quotes:{},confirmLost:true,holdingsFail:false};
  client.defaults.adapter=async config => {
    const f=window.__tokenQA; const path=new URL(config.url,location.origin).pathname; let data={};
    if (config.method !== 'get') f.writes.push({path,body:typeof config.data==='string'?JSON.parse(config.data):config.data});
    if(path.endsWith('/new-coins/NOVA')) data=f.project;
    else if(path.endsWith('/new-coins')) data={projects:[f.project]};
    else if(path.endsWith('/new-coins/holdings')) {if(f.holdingsFail)throw new Error('Fixture unavailable');data={holdings:config.params?.before_id?[]:[f.holding],checked_at:Date.now(),next_before_id:config.params?.before_id?null:7001};}
    else if(path.endsWith('/new-coins/NOVA/quotes')) {const req=JSON.parse(config.data);data=f.quotes[req.idempotency_key];if(!data){const sell=req.side==='sell';data={id:req.idempotency_key,project_id:700,side:req.side,holding_id:sell?7001:null,quantity:sell?'2500':req.quantity,price:'0.4',gross:sell?'1000':'4',cost:sell?'500':'0',fee:sell?'100':'0',net:sell?'900':'4',observed_at:Date.now(),expires_at:Date.now()+15000,status:'quoted',receipt_json:null}; f.quotes[req.idempotency_key]=data;}}
    else if(path.includes('/new-coins/quotes/') && path.endsWith('/confirm')) {const id=path.split('/').at(-2);const q=f.quotes[id];data=q.receipt_json||{quote_id:q.id,project_id:700,side:q.side,quantity:q.quantity,price:q.price,gross:q.gross,fee:q.fee,net:q.net,holding_id:7001,executed_at:Date.now()};q.status='executed';q.receipt_json=data;if(f.confirmLost){f.confirmLost=false;throw new Error('Fixture committed response lost');}}
    else if(path.endsWith('/new-coins/NOVA/subscriptions')){const req=JSON.parse(config.data);data={idempotency_key:req.idempotency_key,status:'pending',lock_position_id:null};}
    else if(path.includes('/wallet'))data={accounts:[]};
    else if(path.endsWith('/klines'))data={klines:Array.from({length:30},(_,i)=>({symbol:'NOVAUSDT',open_time:now-60000*(30-i),open:'0.3',high:'0.41',low:'0.28',close:String(.3+i*.003),volume:'100',provider:'strategy',source:'default'}))};
    else if(path.includes('/new-coins/'))data={subscriptions:[],distributions:[],purchases:[],unlocks:[]};
    return {data,status:200,statusText:'OK',headers:{},config};
  };
  persistAuthTokens('token-issuance-local-fixture','token-fixture-refresh');
  await router.push({name:'new-coin-detail',params:{symbol:'NOVA'}});
});
await page.waitForSelector('.token-holding');
console.log(await page.snapshot({scope:'full_page'}));
console.log(await page.screenshot({path:'/tmp/token-mobile-purchase.png',fullPage:true}));
