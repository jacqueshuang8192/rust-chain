import { Button, Radio, RadioGroup, Space } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { apiRequest } from '../../api/client';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminTextInput } from '../../shared/SemiFormControls';
import { TimestampText } from '../../shared/TimestampText';
import { useCanAdminRequest } from '../access';
import { DefaultMarketForm } from '../resources/actions/defaultMarket/DefaultMarketForm';
import { DefaultMarketPreview } from '../resources/actions/defaultMarket/DefaultMarketPreview';
import { DefaultMarketSelect } from '../resources/actions/defaultMarket/DefaultMarketSelect';
import { DefaultMarketFollowStatus } from '../resources/actions/defaultMarket/DefaultMarketFollowStatus';
import { useDefaultMarketReferences, validateDefaultMarketReference } from '../resources/actions/defaultMarket/referencePairs';
import { UnsavedChangesGuard } from '../settings/UnsavedChangesGuard';
import { checkMarketSetup, marketChanges, marketStatus, volatilityPresets, type MarketDraft, type MarketPair } from './marketSetupModel';
import { projectQueryKey, type NewCoinProject } from './projectModel';
import { useNewCoinMarket } from './useNewCoinMarket';
import './NewCoinMarketSetup.css';

export function NewCoinMarketSetup({ project, onDirty }: { project: NewCoinProject; onDirty: (value: boolean) => void }) {
  const endpoint = `/admin/api/v1/new-coins/${project.id}/market-setup`;
  const canReadProject = useCanAdminRequest(endpoint, 'GET');
  const canReadMarket = useCanAdminRequest('/admin/api/v1/market-pairs', 'GET');
  const canRead = canReadProject && canReadMarket;
  const canWriteProject = useCanAdminRequest(endpoint, 'PATCH');
  const canWriteMarket = useCanAdminRequest('/admin/api/v1/market-pairs', 'PATCH');
  const [selected, setSelected] = useState('');
  const [dirty, setDirty] = useState(false);
  const [generation, setGeneration] = useState(0);
  const query = useQuery({
    queryKey: [...projectQueryKey(project.id), 'market-setup'],
    queryFn: async ({ signal }) => checkMarketSetup(await apiRequest(endpoint, { signal }), project.id),
    enabled: canRead, retry: false, refetchOnWindowFocus: false, refetchOnReconnect: false,
  });
  const setup = query.data;
  const selectedId = project.issuance_policy ? String(project.post_listing_pair_id ?? '') : selected || String(setup?.draft?.pair_id ?? '');
  const pair = setup?.pairs.find(item => String(item.id) === selectedId);
  const writable = canWriteProject && canWriteMarket && project.status === 'active';
  const changed = useCallback((value: boolean) => { setDirty(value); onDirty(value); }, [onDirty]);
  async function reload() {
    const result = await query.refetch();
    if (result.error) throw result.error;
    setGeneration(value => value + 1); setDirty(false); onDirty(false);
  }
  if (!canRead) return <p role="status">缺少行情读取权限。</p>;
  if (!setup) return <section className="new-coin-market">
    <p role={query.error ? 'alert' : 'status'}>{query.error ? adminErrorMessage(query.error, '项目行情加载失败') : '正在读取项目行情'}</p>
    {query.error ? <Button onClick={() => void reload().catch(() => undefined)}>重新加载行情</Button> : null}
  </section>;
  return <section className="new-coin-market" aria-label="项目行情设置">
    {query.error ? <p role="alert">{adminErrorMessage(query.error, '重新加载失败，当前编辑内容已保留')}</p> : null}
    <div className="new-coin-market-heading">
      <h3>{project.symbol} · 行情</h3>
      <Button aria-label="重新加载项目行情" title="重新加载项目行情" icon={<IconRefresh />} disabled={dirty || query.isFetching} onClick={() => void reload().catch(() => undefined)} />
    </div>
    {project.issuance_policy ? <p>行情使用发行合同固定的交易对和认购初价；认购节点启动后可调整生成参数，上市延续价格与历史 K 线。</p> : null}
    {!writable ? <p role="status">当前项目状态或权限下仅可查看。</p> : null}
    {!project.quote_asset_id ? <p role="alert">项目未配置计价资产，暂不能关联行情。</p> : null}
    <div className="new-coin-market-pair">
      <DefaultMarketSelect label="行情交易对" disabled={Boolean(project.issuance_policy) || dirty || query.isFetching} value={selectedId}
        placeholder="选择项目资产对应的交易对" optionList={setup.pairs.map(item => ({
          value: String(item.id), label: `${item.symbol} · ${item.market_type === 'external' ? '外部真实行情' : item.status === 'active' ? '已启用' : '未启用'}`,
        }))} onChange={setSelected} />
      <Link to="/admin/market/pairs">交易对管理</Link>
    </div>
    {setup.pairs.length === 0 ? <p role="status">暂无匹配的交易对。创建时需使用本项目的发行资产和计价资产。</p> : null}
    {selectedId && !pair ? <p role="alert">已关联的交易对不在当前候选目录中，请核对资产配置。</p> : null}
    {pair?.market_type === 'external' ? <p role="status">该交易对使用外部真实行情，请在行情源管理中维护，不使用平台生成参数。</p> : null}
    {pair && pair.market_type !== 'external' ? <MarketWorkspace key={`${project.id}:${pair.id}:${generation}`} project={project} pair={pair}
      saved={setup.draft} writable={writable} onDirty={changed} onReload={reload} /> : null}
  </section>;
}

function MarketWorkspace({ project, pair, saved, writable, onDirty, onReload }: {
  project: NewCoinProject; pair: MarketPair; saved: MarketDraft | null; writable: boolean;
  onDirty: (value: boolean) => void; onReload: () => Promise<void>;
}) {
  const state = useNewCoinMarket(project.id, pair, saved, project.issuance_policy?.purchase_initial_price);
  const [step, setStep] = useState(1);
  const references = useDefaultMarketReferences(writable && state.draft?.mode === 'follow');
  const { draft, live } = state;
  const beforePurchase = Boolean(project.issuance_policy) && (!['purchase', 'listed'].includes(project.lifecycle_status) || !project.actual_purchase_at);
  const blocked = !writable || state.loading || state.busy || state.conflict || state.uncertain;
  const referenceError = draft ? validateDefaultMarketReference(draft, String(pair.id), references, true) : '';
  const invalid = state.invalid || referenceError;
  useEffect(() => { onDirty(state.dirty || state.busy); }, [onDirty, state.dirty, state.busy]);
  const safePreview = () => { if (!blocked && !invalid) void state.runPreview().catch(() => undefined); };
  if (state.loading) return <p role="status">正在读取行情配置</p>;
  if (!draft || !live) return <p role="alert">{state.error || '行情配置不可用，请重新加载'}</p>;
  const field = (key: 'initial_price' | 'price_min' | 'price_max' | 'follow_multiplier' | 'follow_max_move_ratio' | 'follow_stale_after_seconds', label: string) =>
    <label>{label}<AdminTextInput ariaLabel={label} value={draft[key]} disabled={blocked || (key === 'initial_price' && Boolean(project.issuance_policy))} onChange={value => state.edit(key, value)} /></label>;
  return <>
    <UnsavedChangesGuard enabled={state.dirty || state.busy} />
    <section className="new-coin-market-status" aria-label="行情运行状态">
      <dl>
        <div><dt>读取时状态</dt><dd>{marketStatus(live, pair, state.checkedAt)}</dd></div>
        <div><dt>最近运行价格</dt><dd>{live.runtime.last_price ?? '暂无'}</dd></div>
        <div><dt>最后更新</dt><dd><TimestampText value={live.runtime.last_tick_at} /></dd></div>
        <div><dt>线上配置</dt><dd>V{live.version} · {live.enabled ? '默认行情已启用' : '默认行情未启用'}</dd></div>
      </dl>
      <p>状态读取时间：<TimestampText value={state.checkedAt} /></p>
      <Space wrap>
        <Button aria-label="刷新行情状态" title="刷新行情状态" icon={<IconRefresh />} disabled={state.busy} onClick={() => void state.refreshStatus().catch(() => undefined)} />
        {writable ? <ConfirmAction key="pause" actionText={live.all_market_paused ? '恢复行情生成' : '暂停全部行情'} title="确认变更全部行情暂停状态"
          disabled={state.busy || state.dirty || state.conflict || state.uncertain} dangerous={!live.all_market_paused}
          description="只控制行情生成，不撤单、不平仓、不停用交易，也不删除历史行情。" onConfirm={state.pause} /> : null}
        <Link to="/admin/market/strategies">高级策略</Link>
      </Space>
      {live.runtime.error_message ? <p role="alert">{adminErrorMessage(live.runtime.error_message, '行情运行异常')}</p> : null}
      <DefaultMarketFollowStatus value={live} />
    </section>
    {state.error ? <p role="alert">{state.error}</p> : null}
    {state.notice ? <p role="status">{state.notice}</p> : null}
    {state.uncertain ? <div role="alert">请求结果尚未确认，继续操作前请重新查询。<Button disabled={state.busy} onClick={() => void onReload().catch(() => undefined)}>查询最新草稿与发布状态</Button></div> : null}
    {state.conflict ? <div role="alert"><p>草稿或线上版本已变化，当前草稿不会覆盖新配置。</p>
      <ConfirmAction actionText="放弃编辑并加载最新草稿" title="放弃本地编辑并重新核对版本"
        description="将重新读取服务器保存的草稿和线上配置，本地未保存内容会丢弃。" disabled={state.busy} onConfirm={onReload} />
    </div> : null}
    <Space wrap>
      <span>草稿修订：{state.saved?.revision ?? '未保存'}{state.saved?.published_version ? ` · 已发布 V${state.saved.published_version}` : ''}</span>
      {writable ? <ConfirmAction key="adopt" actionText="以线上配置重新编辑" title="放弃当前编辑内容并采用已读取的线上配置"
        disabled={state.busy || state.uncertain} description={`将采用已读取的 V${live.version}。保存时仍会检查版本；不会自动覆盖其他管理员的变更。`}
        onConfirm={() => state.startFromLive()} /> : null}
    </Space>
    <ol className="new-coin-market-steps" aria-label="行情配置步骤">
      {['关联交易对', '选择模式', '配置参数', '预览确认'].map((title, index) =>
        <li key={title} aria-current={index === step ? 'step' : undefined} data-complete={index < step}>
          <span>{index + 1}</span>{title}
        </li>)}
    </ol>
    {step === 0 ? <section><h4>关联交易对</h4><p>{pair.symbol} · 价格精度 {pair.price_precision} 位 · 数量精度 {pair.qty_precision} 位</p>
      <p>{project.issuance_policy ? '交易对随发行合同固定，认购与上市使用同一行情。' : '行情关联独立于上市后购买，不改变项目上市状态或交易开关。'}</p></section> : null}
    {step === 1 ? <section><h4>选择行情模式</h4>
      <RadioGroup type="button" aria-label="行情模式" value={draft.mode} disabled={blocked} onChange={event => state.edit('mode', event.target.value as 'independent' | 'follow')}>
        <Radio value="independent">独立生成</Radio><Radio value="follow">跟随参考行情</Radio>
      </RadioGroup>
      <p>{draft.mode === 'follow' ? '参考其他交易对的相对变化，仍属于平台生成行情，不代表新币在外部市场的实际成交。' : '平台生成行情，不代表真实用户成交或真实流动性。'}</p>
    </section> : null}
    {step === 2 ? <section><h4>配置核心参数</h4><div className="admin-action-form">
      {field('initial_price', '初始价格')}
      <DefaultMarketSelect label="波动程度" value={draft.volatility} disabled={blocked}
        optionList={[...volatilityPresets, ...(!volatilityPresets.some(item => item.value === draft.volatility) ? [{ value: draft.volatility, label: `自定义 · ${draft.volatility}` }] : [])]}
        onChange={value => state.edit('volatility', value)} />
      {field('price_min', '价格下限（可选）')}{field('price_max', '价格上限（可选）')}
      {draft.mode === 'follow' ? <>
        <DefaultMarketSelect label="参考交易对" disabled={blocked || references.loading} value={draft.reference_pair_id} filter
          optionList={references.data.map(item => ({ value: String(item.id), label: item.symbol }))} onChange={value => state.edit('reference_pair_id', value)} />
        {field('follow_multiplier', '跟随强度')}
      </> : null}
    </div>
      <p>初始价只在没有历史价格时使用；已有行情按引擎规则续接，不以发行价重置。</p>
      {writable && !project.issuance_policy && !draft.initial_price && !live.runtime.last_price ? <Button disabled={blocked}
        onClick={() => state.edit('initial_price', project.issue_price)}>填入发行价 {project.issue_price}</Button> : null}
      <details className="new-coin-market-advanced"><summary>高级参数</summary>
        <DefaultMarketForm draft={draft} edit={state.edit} disabled={blocked} pricePrecision={pair.price_precision} qtyPrecision={pair.qty_precision} hideEnable hidePriceFields />
        {draft.mode === 'follow' ? <div className="admin-action-form">{field('follow_max_move_ratio', '每分钟变动上限（比例）')}{field('follow_stale_after_seconds', '参考过期秒数')}</div> : null}
      </details>
    </section> : null}
    {step === 3 ? <section><h4>预览与发布</h4>
      <p>{pair.symbol} · {draft.mode === 'follow' ? '跟随参考行情' : '独立生成'} · 目标版本 V{state.baseVersion + 1}</p>
      <dl className="new-coin-market-changes" aria-label="配置变更">
        {marketChanges(live, draft).map(change => <div key={change.label}><dt>{change.label}</dt><dd>{change.before} → {change.after}</dd></div>)}
      </dl>
      <p>手工策略优先；全部暂停时发布不会自动恢复。生成价格可能影响现货触发、杠杆估值和秒合约取价，实际依赖需在业务记录中核对。</p>
      {beforePurchase ? <p role="status">尚未进入认购，可以保存与预览；行情由认购节点启动，无需提前启用交易对。</p> : pair.status !== 'active' ? <p role="alert">交易对未启用，可以保存和预览；发布前需在交易对管理中单独确认启用。</p> : null}
      <Button disabled={blocked || Boolean(invalid)} loading={state.busy} onClick={safePreview}>生成行情预览</Button>
      {state.preview ? <DefaultMarketPreview preview={state.preview} requestedMode={draft.mode} /> : null}
    </section> : null}
    {invalid ? <p role="alert">{invalid}</p> : null}
    <div className="new-coin-market-actions">
      <Space wrap>
        <Button disabled={step === 0 || state.busy} onClick={() => setStep(value => value - 1)}>上一步</Button>
        {step < 3 ? <Button disabled={state.busy || (step === 2 && Boolean(invalid))} onClick={() => setStep(value => value + 1)}>下一步</Button> : null}
      </Space>
      {writable ? <Space wrap>
        <ConfirmAction key="save" actionText="保存草稿" title={`保存 ${pair.symbol} 行情草稿`}
          description="仅保存项目草稿，不改变线上生成器、交易开关或上市状态。"
          disabled={blocked || Boolean(invalid) || (!state.dirty && state.saved?.pair_id === pair.id)} onConfirm={reason => {
            if (blocked || invalid) throw new Error(invalid || '当前不可保存'); return state.save(reason);
          }} />
        {step === 3 ? <ConfirmAction key="publish" actionText="确认发布行情" title={`发布 ${pair.symbol} 默认行情 V${state.baseVersion + 1}`}
          description="将启用并更新默认生成器。手工策略和全部暂停仍优先；不更改真实订单、历史K线、交易对状态或上市后购买开关。"
          disabled={blocked || beforePurchase || Boolean(invalid) || state.dirty || !state.preview || !state.saved || state.saved.pair_id !== pair.id || state.saved.published_version !== null || pair.status !== 'active'}
          onConfirm={reason => { if (blocked || invalid) throw new Error(invalid || '当前不可发布'); return state.publish(reason); }} /> : null}
      </Space> : null}
    </div>
  </>;
}
