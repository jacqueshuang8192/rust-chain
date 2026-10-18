import { describe, expect, it } from 'vitest';
import { allocationPreview, canAllocate } from './AllocationPanel';
import { buildPolicy, emptyPolicyDraft, validatePolicy, policyIdentity } from './issuancePolicy';
import type { AgentTeamOrder } from '../api/agentTeamOrders';
const order = { id:1, user_id:2, email:null, owner_agent_id:3, owner_agent_code:'A', status:'pending', created_at:1900000000000, issuance_version:2, project_lifecycle_status:'distribution', settlement_mode:'manual_distribution', base_asset_precision:8, quote_asset_precision:8, requested_quantity:'5000', issue_price:'0.2', quote_amount:'1000', symbol:'N', quote_symbol:'USDT' } as AgentTeamOrder;
describe('versioned issuance contract', () => {
  it('preserves independent node switches and local-time instants in the reviewed contract', () => {
    const draft=emptyPolicyDraft(); draft.purchasePrice='0.3'; draft.listingPrice='9.99'; draft.lock='months'; draft.period='3'; draft.fee='profit'; draft.feeValue='0.2';
    for(const [i,node] of Object.values(draft.nodes).entries()) { node.local=`2030-01-0${i+1}T11:00`; node.automatic=i%2===0; }
    const p=buildPolicy(draft,8); expect(p.purchase_initial_price).toBe('0.3'); expect(p.subscription.at).toBe(new Date('2030-01-01T11:00').getTime()); expect(p.subscription.automatic).toBe(true); expect(p.distribution.automatic).toBe(false); expect(p.lock).toEqual({kind:'months',count:3});
    expect(()=>validatePolicy({...p,purchase:{...p.purchase,at:p.distribution.at}})).toThrow();
    expect(policyIdentity(p)).not.toBe(policyIdentity({...p,lock:{kind:'none'}}));
    expect(()=>validatePolicy({...p,fee:{kind:'gross',rate:'1.01'}})).toThrow();
  });
  it.each([['50','2500','500'],['80','4000','200'],['100','5000','0'],['0','0','1000']])('previews %s percent with exact original-price refund', (percent,quantity,refund)=>{
    expect(allocationPreview(order,percent)).toMatchObject({quantity,refund});
  });
  it('rounds down at token precision and rejects old, terminal or invalid batches',()=>{
    expect(allocationPreview({...order,base_asset_precision:0,requested_quantity:'3',quote_amount:'0.6'},'50')).toMatchObject({quantity:'1',payment:'0.2',refund:'0.4'});
    expect(canAllocate({...order,issuance_version:1})).toBe(false);
    expect(canAllocate({...order,status:'allocated'})).toBe(false);
    expect(()=>allocationPreview(order,'100.1')).toThrow();
    expect(()=>allocationPreview({...order,base_asset_precision:undefined},'80')).toThrow();
  });
});
