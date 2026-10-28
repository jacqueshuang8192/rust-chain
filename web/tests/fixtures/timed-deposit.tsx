// Local-only fixture: actual editor, permissions, API client and review dialogs.
import '@douyinfe/semi-ui/react19-adapter';
import '@douyinfe/semi-ui/lib/es/_base/base.css';
import '../../src/styles.css';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { AdminAccessProvider } from '../../src/admin/access';
import { authStore } from '../../src/auth/authStore';
import { DepositSettingsPage } from '../../src/admin/deposits/DepositSettingsPage';
import { DepositReviewPage } from '../../src/admin/deposits/DepositReviewPage';

const now=Date.now();
const address='0x1111111111111111111111111111111111111111';
const progress = { last_scan_at:now,last_error:null as string|null,scan_fresh:true,cursor_height:120,recovery_mode:null as string|null,backfill_cursor_height:null as number|null,backfill_target_height:null as number|null,backfill_last_error:null as string|null,backfill_blocked:false };
const scanner = { ...progress,network:'eth',providers:[],streams:[] as Array<typeof progress & {stream:string}>,enabled:true,endpoint_configured:true,api_key_configured:true,confirmations:12,revision:1,checked_at:now };
const settings={
 scanners:[scanner,{...scanner,network:'base',confirmations:1,last_error:'ETH 原生币: RPC 方法不可用',scan_fresh:false,streams:[{...progress,stream:'native',cursor_height:100,last_error:'RPC 方法 eth_getTransactionByHash 失败，请检查渠道权限和额度',scan_fresh:false},{...progress,stream:'token',backfill_cursor_height:80,backfill_target_height:110}]}],
 assets:[{id:1,network:'eth',asset_symbol:'ETH',asset_kind:'native',token_address:'',decimals:18,enabled:true,revision:1},{id:2,network:'base',asset_symbol:'ETH',asset_kind:'native',token_address:'',decimals:18,enabled:true,revision:1},{id:3,network:'base',asset_symbol:'USDC',asset_kind:'token',token_address:'0x833589fcd6edb6e08f4c7c32d4f71b54bda02913',decimals:6,enabled:true,revision:1}],
 addresses:[{pool_id:1,network:'eth',address,enabled:true},{pool_id:2,network:'base',address,enabled:true}],
 platform_assets:[{symbol:'ETH',status:'active',deposit_enabled:true,precision_scale:18},{symbol:'USDC',status:'active',deposit_enabled:true,precision_scale:6}]
};
const order={id:1,order_no:'CD-local-payment-review',user_id:5,asset_symbol:'ETH',network:'eth',asset_kind:'native',token_address:'',address,amount:'100',fee_amount:'0.01',paid_amount:null as string|null,status:'review',created_at:now-20000,expires_at:now+1780000,required_confirmations:12,tx_hash:null};
const receipt={id:1,network:'eth',tx_hash:'0xlocal-payment-evidence',event_index:2147483648,asset_kind:'native',token_address:'',address,amount:'100',block_height:200,block_hash:'0xlocal-block',block_time:now-10000,confirmations:12,status:'review',review_reason:'expired_unpaid_combination_reused',order_id:1,review_note:null as string|null};
window.fetch=async(input,init)=>{
 const path=new URL(String(input),location.origin).pathname;
 if(path.endsWith('/deposit-settings/scanners/eth')&&init?.method==='PUT'){const body=JSON.parse(String(init.body));Object.assign(settings.scanners[0],{enabled:body.enabled,confirmations:body.confirmations,revision:2});return new Response(null,{status:204});}
 if(path.endsWith('/deposit-receipts/1/review')){const body=JSON.parse(String(init?.body));receipt.status=body.action==='credit'?'credited':'ignored';receipt.review_note=body.reason;if(body.action==='credit'){order.status='credited';order.paid_amount=receipt.amount;}return new Response(null,{status:204});}
 if(path.endsWith('/deposit-settings/scanners/base/recovery')){
  const body=JSON.parse(String(init?.body));
  if(body.expected_revision!==settings.scanners[1].revision)return Response.json({message:'stale revision'},{status:409});
  const target=settings.scanners[1].streams.find(s=>s.stream===body.stream);
  if(!target)return Response.json({message:'select a stream'},{status:400});
  target.recovery_mode=body.mode;settings.scanners[1].revision+=1;
  document.documentElement.dataset.recoveryStream=body.stream;
  return Response.json({queued:true},{status:202});
 }
 if(path.endsWith('/deposit-settings'))return Response.json(settings);
 if(path.endsWith('/deposit-receipts'))return Response.json({items:receipt.status==='review'?[receipt]:[],total:receipt.status==='review'?1:0});
 if(path.endsWith('/deposit-orders/1'))return Response.json(order);
 if(path.endsWith('/deposit-orders'))return Response.json({items:[order],total:1});
 return Response.json({message:'Unexpected fixture request'},{status:404});
};
authStore.setSession({accessToken:'local-only',refreshToken:'local-refresh',scope:'admin',subject:'admin:1'});
const client=new QueryClient({defaultOptions:{queries:{retry:false}}});
const router=createMemoryRouter([{path:'/',element:<DepositSettingsPage/>},{path:'/admin/wallet/deposit-settings',element:<DepositSettingsPage/>},{path:'/admin/wallet/deposit-receipts',element:<DepositReviewPage/>}]);
createRoot(document.getElementById('root')!).render(<AdminAccessProvider access={{admin_id:1,username:'local-reviewer',role_id:1,role_name:'Local',is_super_admin:true,permissions:['*']}}><QueryClientProvider client={client}><div style={{padding:24,maxWidth:1200,margin:'auto'}}><RouterProvider router={router}/></div></QueryClientProvider></AdminAccessProvider>);
