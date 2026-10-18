import { TimestampText } from '../shared/TimestampText';
import { multiplyDecimalText } from '../shared/decimal';
import { lockLabel, phaseLabels, phaseNames, type IssuancePolicy } from './issuancePolicy';
export function IssuancePolicySummary({ policy, quote = '' }: { policy: IssuancePolicy; quote?: string }) {
  return <div className="issuance-summary">
    <p>配置时区：{policy.timezone}；以下时间按您当前时区显示。</p>
    <dl><dt>认购初价</dt><dd>{policy.purchase_initial_price} {quote} · 后续按实时行情成交</dd><dt>上市参考价</dt><dd>{policy.listing_reference_price} {quote} · 不重置行情</dd>
      {phaseNames.map(k => <div key={k} className="issuance-summary-row"><dt>{phaseLabels[k]}</dt><dd><TimestampText value={policy[k].at} /> · {policy[k].automatic ? '自动' : '手动'}</dd></div>)}
      <dt>锁仓规则</dt><dd>{lockLabel(policy.lock)}</dd><dt>退出费用</dt><dd>{policy.fee.kind === 'none' ? '不收费' : policy.fee.kind === 'fixed' ? `${policy.fee.amount} ${quote}` : `${policy.fee.kind === 'profit' ? '正收益' : '卖出总价'} × ${multiplyDecimalText(policy.fee.rate, '100')}%`}</dd>
    </dl><p>派发由代理分批确认；解锁卖出与净额到账一次完成。</p>
  </div>;
}
