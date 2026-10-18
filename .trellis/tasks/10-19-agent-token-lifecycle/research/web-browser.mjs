const task=await taskSpace(22);const p=await task.newPage();console.log('webPage',p.label);await p.goto('http://localhost:3030');
await p.evaluate(async()=>{
 const now=Date.now();const policy={version:2,timezone:'Asia/Hong_Kong',purchase_initial_price:'0.3',listing_reference_price:'0.5',subscription:{at:now-86400000*4,automatic:true},distribution:{at:now-86400000*3,automatic:false},purchase:{at:now+86400000,automatic:true},listing:{at:now+86400000*7,automatic:false},lock:{kind:'on_purchase'},fee:{kind:'profit',rate:'0.2'}};
 const application={id:700,agent_id:3,agent_admin_id:8,symbol:'NOVA',name:'Nova Token',precision_scale:8,total_supply:'20000',issue_price:'0.2',quote_asset_id:702,quote_symbol:'USDT',listed_at:policy.listing.at,reason:'Local issuance acceptance',status:'pending',project_id:null,reviewed_by:null,review_reason:null,reviewed_at:null,created_at:now,issuance_policy:policy};
 const row=id=>({id,user_id:id+10,email:`buyer${id}@example.test`,owner_agent_id:3,owner_agent_code:'TEAM-A',status:'pending',created_at:now,issuance_version:2,project_lifecycle_status:'distribution',settlement_mode:'manual_distribution',base_asset_precision:8,quote_asset_precision:8,requested_quantity:'5000',issue_price:'0.2',quote_amount:'1000',symbol:'NOVA',quote_symbol:'USDT',project_id:700,quote_asset:702,frozen_quote_amount:'1000',settled_quote_amount:null,refunded_quote_amount:null,allocated_quantity:'0'});
 window.__tokenWeb={application,policy,rows:[row(71),row(72)],writes:[],reads:[],project:{id:700,symbol:'NOVA',lifecycle_status:'distribution',status:'active',issuer_agent_id:3,issuance_policy:policy,actual_purchase_at:null,actual_listed_at:null,next_transition_at:null,transition_error:null}};
 const native=window.fetch.bind(window);window.fetch=async(input,init={})=>{
  const url=new URL(typeof input==='string'?input:input.url,location.href);if(!url.pathname.includes('/api/'))return native(input,init);
  const f=window.__tokenWeb;const path=url.pathname;const method=init.method||'GET';let data={};const body=init.body?JSON.parse(init.body):null;
  if(method!=='GET')f.writes.push({path,body});else f.reads.push(path);
  if(path.endsWith('/access/me'))data={admin_id:1,username:'Local Reviewer',role_id:1,role_name:'Admin',permissions:['*'],is_super_admin:true};
  else if(path.endsWith('/auth/me'))data={agent_admin_id:8,agent_id:3,username:'Local Agent',agent_code:'TEAM-A',level:1,agent_status:'active',admin_status:'active'};
  else if(path.endsWith('/coin-applications/options'))data=[{asset_id:702,symbol:'USDT',precision_scale:8}];
  else if(path.endsWith('/review')){f.application={...f.application,status:body.decision==='approve'?'approved':'rejected',project_id:body.decision==='approve'?700:null,reviewed_by:1,review_reason:body.reason,reviewed_at:Date.now()};data=f.application;}
  else if(/coin-applications\/\d+$/.test(path))data=f.application;
  else if(path.endsWith('coin-applications')){if(method==='POST'){f.application={...f.application,...body};data=f.application;}else data={applications:[f.application],total:1};}
  else if(path.endsWith('/stage')){f.project.lifecycle_status=body.target_stage;data=f.project;}
  else if(path.endsWith('/token-projects'))data={projects:url.searchParams.has('before_id')?[]:[f.project],next_before_id:url.searchParams.has('before_id')?null:700};
  else if(path.endsWith('/new-coin-allocations')){data={idempotency_key:body.idempotency_key,results:body.subscription_ids.map(id=>({subscription_id:id,quantity:body.percent==='50'?'2500':'4000',settled_quote_amount:body.percent==='50'?'500':'800',refunded_quote_amount:body.percent==='50'?'500':'200',holding_id:id,status:'held'}))};f.rows=f.rows.filter(r=>!body.subscription_ids.includes(r.id));}
  else if(path.endsWith('/orders/new-coin-subscriptions'))data={orders:f.rows,total:f.rows.length};
  else return new Response(JSON.stringify({message:`Unhandled fixture ${path}`}),{status:404,headers:{'content-type':'application/json'}});
  return new Response(JSON.stringify(data),{status:200,headers:{'content-type':'application/json'}});
 };
 const {authStore}=await import('/src/auth/authStore.ts');authStore.setSession({scope:'agent',subject:'agent:8',accessToken:'token-local-qa',refreshToken:'qa-refresh'});
 const {router}=await import('/src/app/router.tsx');await router.navigate('/agent/coin-applications');
});
await p.waitForSelector('input[aria-label="币种代码"]',{timeout:10000});console.log(await p.snapshot({scope:'full_page'}));
