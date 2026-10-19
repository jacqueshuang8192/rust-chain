import type { ProjectCenter } from '../projectModel';
export function center(stage='preheat', id=7): ProjectCenter {
  return { configuration_version:'snapshot-v1', subscription_count:0,pending_manual_count:0,issuance_editable:stage==='preheat',next_lifecycle_status:({preheat:'subscription',subscription:'distribution',distribution:'listed'} as Record<string,string>)[stage]??null,lifecycle_block_reason:null,
    project:{id,asset_id:11,symbol:'HIP',lifecycle_status:stage,status:'active',quote_asset_id:3,total_supply:'100',issue_price:'2.5',reserved_supply:'0',allocated_supply:'0',remaining_supply:'100',unlock_type:'fixed_time',listed_at:1794309753000,actual_listed_at:stage==='listed'?1794309753250:null,fixed_unlock_at:1794409753250,relative_unlock_seconds:null,unlock_fee_enabled:true,unlock_fee_rate:'0.04',unlock_fee_basis:'market_value',unlock_fee_asset:3,post_listing_purchase_enabled:false,post_listing_pair_id:null} };
}
export const manualOrder={id:91,project_id:7,user_id:42,settlement_mode:'manual_distribution',status:'pending',issue_price:'2.5',quote_amount:'25',frozen_quote_amount:'25',requested_quantity:'10',quote_asset:3};

export function tokenCenter(stage = 'distribution'): ProjectCenter {
  const data = center(stage);
  return { ...data, issuance_editable: false, next_lifecycle_status: stage === 'listed' ? null : stage === 'distribution' ? 'purchase' : stage === 'purchase' ? 'listed' : 'subscription', project: {
    ...data.project, issuer_agent_id: 8, issuer_agent_code: 'AGENT-A', quote_symbol: 'USDT', post_listing_pair_id: 11, post_listing_pair_symbol: 'HIPUSDT',
    actual_purchase_at: stage === 'purchase' || stage === 'listed' ? 1800002000000 : null, next_transition_at: 1800002000000, next_transition_attempt_at: 1800002060000, transition_error: 'transition_blocked',
    issuance_policy: { version: 2, timezone: 'Asia/Hong_Kong', purchase_initial_price: '0.3', listing_reference_price: '9.99',
      subscription: { automatic: true, at: 1800000000000 }, distribution: { automatic: false, at: 1800001000000 }, purchase: { automatic: true, at: 1800002000000 }, listing: { automatic: false, at: 1800003000000 }, lock: { kind: 'on_purchase' }, fee: { kind: 'profit', rate: '0.2' } },
  } };
}
