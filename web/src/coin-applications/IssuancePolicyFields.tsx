import { Switch } from '@douyinfe/semi-ui';
import { AdminSelect, AdminTextInput } from '../shared/SemiFormControls';
import { phaseNames, phaseLabels, type PolicyDraft } from './issuancePolicy';

export function IssuancePolicyFields({ value, onChange, disabled }: { value: PolicyDraft; onChange: (value: PolicyDraft) => void; disabled: boolean }) {
  const set = (key: keyof PolicyDraft, next: string) => onChange({ ...value, [key]: next });
  return <>
    <section className="issuance-section"><h3>认购行情与上市</h3><p>认购初价仅用于启动行情。用户按实时行情成交，上市后沿用同一行情转入现货。</p>
      <div className="coin-application-fields">
        <label>认购初价<AdminTextInput ariaLabel="认购初价" value={value.purchasePrice} disabled={disabled} onChange={v => set('purchasePrice', v)} /></label>
        <label>上市参考价<AdminTextInput ariaLabel="上市参考价" value={value.listingPrice} disabled={disabled} onChange={v => set('listingPrice', v)} /><small>仅作为发行资料，不重置上市成交价。</small></label>
      </div>
    </section>
    <section className="issuance-section"><h3>时间节点</h3><p>当前配置时区：{value.timezone}。其他时区会显示对应当地时间。</p>
      <div className="issuance-timeline">{phaseNames.map(key => <div key={key} className="issuance-node">
        <label>{phaseLabels[key]}<AdminTextInput ariaLabel={phaseLabels[key]} value={value.nodes[key].local} type="datetime-local" disabled={disabled}
          onChange={local => onChange({ ...value, nodes: { ...value.nodes, [key]: { ...value.nodes[key], local } } })} /></label>
        <label className="issuance-switch"><Switch aria-label={`${phaseLabels[key]}自动操作`} checked={value.nodes[key].automatic} disabled={disabled}
          onChange={automatic => onChange({ ...value, nodes: { ...value.nodes, [key]: { ...value.nodes[key], automatic } } })} />{value.nodes[key].automatic ? '到时自动推进' : '手动推进'}</label>
      </div>)}</div><small>进入派发阶段只开放操作。代理需要选择下级订单，分批设置比例并确认派发。</small>
    </section>
    <section className="issuance-section"><h3>锁仓与卖出</h3><div className="coin-application-fields">
      <label id="issuance-lock-label">锁仓规则<AdminSelect ariaLabel="锁仓规则" ariaLabelledBy="issuance-lock-label" value={value.lock} disabled={disabled} onChange={v => set('lock', v)}
        optionList={[{ value: 'none', label: '不锁仓' }, { value: 'on_purchase', label: '认购开始即解禁' }, { value: 'on_listing', label: '上市即解禁' }, { value: 'weeks', label: '购买后按周解禁' }, { value: 'months', label: '购买后按自然月解禁' }, { value: 'fixed', label: '固定时间解禁' }]} /></label>
      {value.lock === 'weeks' || value.lock === 'months' ? <label>锁仓周期（{value.lock === 'weeks' ? '周' : '月'}）<AdminTextInput ariaLabel="锁仓周期" value={value.period} disabled={disabled} onChange={v => set('period', v)} /></label> : null}
      {value.lock === 'fixed' ? <label>固定解锁时间<AdminTextInput ariaLabel="固定解锁时间" type="datetime-local" value={value.unlockAt} disabled={disabled} onChange={v => set('unlockAt', v)} /></label> : null}
      <label id="issuance-fee-label">解锁卖出手续费<AdminSelect ariaLabel="解锁卖出手续费" ariaLabelledBy="issuance-fee-label" value={value.fee} disabled={disabled} onChange={v => set('fee', v)}
        optionList={[{ value: 'none', label: '不收费' }, { value: 'profit', label: '按正收益比例' }, { value: 'gross', label: '按卖出总价比例' }, { value: 'fixed', label: '固定计价资产金额' }]} /></label>
      {value.fee !== 'none' ? <label>{value.fee === 'fixed' ? '固定费用' : '费用比例（0.1 表示 10%）'}<AdminTextInput ariaLabel="退出费用" value={value.feeValue} disabled={disabled} onChange={v => set('feeValue', v)} /></label> : null}
    </div><p>到期后由用户确认解锁卖出，平台按有效行情回购，手续费从卖出款中扣除并一次到账。</p></section>
  </>;
}
