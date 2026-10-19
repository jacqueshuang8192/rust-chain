import { Button, Card, Space, Tabs, TabPane, Typography } from '@douyinfe/semi-ui';
import { useCallback, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, useParams } from 'react-router-dom';
import { apiRequest, ApiError } from '../../api/client';
import { PageHeader } from '../../layouts/PageHeader';
import { TimestampText } from '../../shared/TimestampText';
import { AmountText } from '../../shared/AmountText';
import { StatusTag } from '../../shared/StatusTag';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminRequestActionBoundary, useCanAdminRequest } from '../access';
import { ADMIN_OPTION_QUERY_KEY } from '../sharedOptionQuery';
import { loadProjectCenter, projectQueryKey, stages } from './projectModel';
import { NewCoinProjectSettings } from './NewCoinProjectSettings';
import { NewCoinGrant } from './NewCoinGrant';
import { NewCoinMarketSetup } from './NewCoinMarketSetup';

function RecordLink({ path, endpoint: explicitEndpoint, children }: { path:string; endpoint?: string; children:React.ReactNode }) {
  const endpoint=explicitEndpoint ?? path.split('?')[0].replace('/admin/','/admin/api/v1/');
  const canRead=useCanAdminRequest(endpoint,'GET');
  return canRead ? <Link to={path}>{children}</Link> : null;
}

export function NewCoinProjectPage() {
  const { projectId = '' }=useParams();
  const queryClient=useQueryClient();
  const [dirty,setDirty]=useState(false);
  const [activeTab,setActiveTab]=useState('overview');
  const [error,setError]=useState('');
  const [phaseBusy,setPhaseBusy]=useState(false);
  const [uncertain,setUncertain]=useState(false);
  const [revision,setRevision]=useState(0);
  const query=useQuery({queryKey:projectQueryKey(projectId),queryFn:({signal})=>loadProjectCenter(projectId,signal),enabled:/^[1-9]\d*$/.test(projectId),retry:false,refetchOnWindowFocus:false});
  const canWrite=useCanAdminRequest(`/admin/api/v1/new-coins/${projectId}/issuance`,'PATCH');
  const reload=useCallback(async()=>{
    const result = await query.refetch();
    if (result.error) throw result.error;
    setRevision(v=>v+1); setDirty(false); setUncertain(false); setError('');
    void queryClient.invalidateQueries({predicate:q=>q.queryKey[0]===ADMIN_OPTION_QUERY_KEY&&q.queryKey[2]==='reference:newCoinProject:100'});
  },[query.refetch,queryClient]);
  const data=query.data;
  if(!/^[1-9]\d*$/.test(projectId)) return <main className="exchange-page"><PageHeader title="项目编号无效" /><Link to="/admin/new-coins/projects">返回项目管理</Link></main>;
  if(!data || query.error) return <main className="exchange-page"><PageHeader title="新币项目中心" />
    <Card><Typography.Text>{query.isFetching?'正在读取项目配置…':query.error?.message ?? '项目不存在'}</Typography.Text><Button onClick={()=>void reload().catch(()=>undefined)}>重新加载项目</Button></Card>
  </main>;
  const p=data.project;
  const stage=stages.find(s=>s.value===p.lifecycle_status);
  const versioned = !!p.issuance_policy;
  const action = versioned && p.lifecycle_status === 'distribution' ? '开始实时认购' : stage?.action;
  return <main className="exchange-page">
    <PageHeader title={`${p.symbol} · 项目中心`} description={`项目 ID ${p.id} · 发行资产 ${p.asset_id} · 计价资产 ${p.quote_asset_id ?? '未配置'}`}
      actions={<Space><Link to="/admin/new-coins/projects">返回项目管理</Link><Button disabled={dirty||phaseBusy||query.isFetching} onClick={()=>void reload().catch(()=>undefined)}>刷新项目</Button></Space>} />
    <Card bordered={false}>
      <Space vertical align="start" spacing={16} style={{width:'100%'}}>
        <Space wrap><StatusTag value={p.lifecycle_status}/><StatusTag value={p.status}/><Typography.Text>{versioned ? '预热中 → 申购中 → 派发中 → 实时认购 → 现货上市' : '预热中 → 申购中 → 派发中 → 已上市'}</Typography.Text></Space>
        <Typography.Text>{versioned ? '申购冻结资金，代理分批派发并退差额；全部结清后开放实时认购，上市时延续同一交易对、行情与历史K线。' : '申购只冻结资金；结束申购后人工派发并退差额，结清待处理订单后才允许上市。项目启停与个人解禁是独立状态。'}</Typography.Text>
        {data.lifecycle_block_reason ? <Typography.Text type="warning">{data.lifecycle_block_reason}</Typography.Text> : null}
        {dirty ? <Typography.Text type="warning">请先保存或重置配置，再推进项目阶段。</Typography.Text> : null}
        {error ? <Typography.Text type="danger">{error}</Typography.Text> : null}
        {data.next_lifecycle_status ? <AdminRequestActionBoundary key="lifecycle-action" endpoint={`/admin/api/v1/new-coins/${p.id}/lifecycle`} method="PATCH">
          <ConfirmAction actionText={action ?? '推进阶段'} disabled={!!data.lifecycle_block_reason||dirty||phaseBusy||uncertain||query.isFetching}
            title={`确认 ${p.symbol} ${action}；${p.lifecycle_status==='subscription'?'结束后停止接受新申购，不自动派发':versioned&&p.lifecycle_status==='distribution'?'启动原交易对行情，用户按实时有效报价认购':p.lifecycle_status==='purchase'?'关闭认购入口，延续行情转入现货':p.lifecycle_status==='distribution'?'以本次操作时间记录上市，不自动启用现货交易对':'开放用户申购并冻结资金'}`}
            onConfirm={async reason=>{
              setPhaseBusy(true);
              try { await apiRequest(`/admin/api/v1/new-coins/${p.id}/lifecycle`,{method:'PATCH',body:JSON.stringify({lifecycle_status:data.next_lifecycle_status,expected_config:data.configuration_version,reason})});setError('');await reload(); }
              catch(cause){setUncertain(true);setError(cause instanceof ApiError&&cause.status===409?'项目已变化或存在待处理申购，请刷新后重试。':'操作结果待核对，请刷新项目后继续。');throw cause;}
              finally {setPhaseBusy(false);}
            }} />
        </AdminRequestActionBoundary> : null}
      </Space>
    </Card>
    <Tabs keepDOM lazyRender activeKey={activeTab} onChange={key=>{if(!dirty&&!phaseBusy)setActiveTab(key);}} tabPaneMotion={false} style={{marginTop:24}}>
      <TabPane itemKey="overview" tab="项目概览">
        <Card bordered={false}><Space vertical align="start" spacing={16}>
          <Typography.Text>发行总量：<AmountText value={p.total_supply}/> · {versioned ? '固定申购价' : '发行价'}：<AmountText value={p.issue_price}/></Typography.Text>
          {versioned ? <>
            <Typography.Text>发行方：{p.issuer_agent_id == null ? '平台直接发行' : `${p.issuer_agent_code || '代理'} · ID ${p.issuer_agent_id}`} · 计价币种：{p.quote_symbol} · 交易对：{p.post_listing_pair_symbol}</Typography.Text>
            <Typography.Text>实际认购开始：{p.actual_purchase_at == null ? '尚未进入认购' : <TimestampText value={p.actual_purchase_at}/>}</Typography.Text>
            <Typography.Text>下一自动节点：{p.next_transition_at == null ? '当前无自动节点，按计划手动推进' : <TimestampText value={p.next_transition_at}/>}</Typography.Text>
            {p.transition_error ? <Typography.Text type="warning">自动节点尚未完成，请核对待结算申购与行情状态。下次尝试：<TimestampText value={p.next_transition_attempt_at ?? null}/></Typography.Text> : null}
          </> : null}
          <Typography.Text>已预留：<AmountText value={p.reserved_supply}/> · 已派发：<AmountText value={p.allocated_supply}/> · 剩余额度：<AmountText value={p.remaining_supply}/></Typography.Text>
          <Typography.Text>计划上市时间：<TimestampText value={p.listed_at}/> · 实际上市时间：{p.actual_listed_at === null ? (p.lifecycle_status === 'listed' ? '历史事件未记录' : '尚未确认上市') : <TimestampText value={p.actual_listed_at}/>}</Typography.Text>
          <Typography.Text>{versioned ? '节点按审核的独立开关自动或手动推进；进入派发阶段不会自动派币，到期只获得卖出资格，不会自动出售。' : '计划时间不自动推进阶段；新增“上市即解禁”仓位须等待实际上市，固定时间和相对周期沿用各自到期规则。'}</Typography.Text>
          <Typography.Text>累计申购订单：{data.subscription_count} · 待派发 / 退款：{data.pending_manual_count}</Typography.Text>
          <Typography.Text>{versioned ? '发行参数与锁仓、退出费用规则已在创建或审核时固定，可在发行合同中查看。' : '资产和计价币种在项目创建后保持不变；发行参数仅在预热且无订单、无额度占用时可修改。'}</Typography.Text>
        </Space></Card>
      </TabPane>
      <TabPane itemKey="settings" tab={versioned?'发行合同':canWrite?'项目配置':'配置详情'}>
        <NewCoinProjectSettings key={`${p.id}:${revision}`} data={data} onSaved={reload} onDirty={setDirty}/>
      </TabPane>
      <TabPane itemKey="market" tab="行情">
        <NewCoinMarketSetup key={`${p.id}:${revision}`} project={p} onDirty={setDirty}/>
      </TabPane>
      <TabPane itemKey="records" tab="关联记录">
        <Card bordered={false}><Space vertical align="start" spacing={16}>
          <RecordLink path={`/admin/new-coins/subscriptions?project_id=${p.id}&status=pending`}>处理待派发申购</RecordLink>
          <RecordLink path={`/admin/new-coins/subscriptions?project_id=${p.id}`}>全部申购与配售</RecordLink>
          <RecordLink path={`/admin/new-coins/distributions?project_id=${p.id}`}>派发与退款记录</RecordLink>
          <RecordLink endpoint={`/admin/api/v1/new-coins/${p.id}/reconciliation`} path={`/admin/new-coins/reconciliation/${p.id}`}>派发与退款批次对账</RecordLink>
          {!versioned ? <><RecordLink path={`/admin/new-coins/lock-positions?asset_id=${p.asset_id}`}>该资产锁仓（含其他项目及来源）</RecordLink>
          <RecordLink path={`/admin/new-coins/unlocks?asset_id=${p.asset_id}`}>该资产解禁记录（含其他项目及来源）</RecordLink></> : <>
            <RecordLink path={`/admin/new-coins/holdings?project_id=${p.id}`}>项目持仓与原始锁仓规则</RecordLink>
            <RecordLink path={`/admin/new-coins/buybacks?project_id=${p.id}`}>解锁回购与净到账记录</RecordLink>
            <RecordLink path={`/admin/new-coins/operations?project_id=${p.id}`}>阶段、派发与成交操作记录</RecordLink>
          </>}
          <RecordLink path={`/admin/new-coins/purchases?project_id=${p.id}`}>{versioned ? '实时认购订单' : '上市后购买订单'}</RecordLink>
          <RecordLink path={`/admin/audit-logs?target_type=new_coin_project&target_id=${p.id}`}>项目配置与生命周期审计</RecordLink>
        </Space></Card>
      </TabPane>
      {canWrite && !versioned ? <TabPane itemKey="grant" tab="额外赠币" disabled={dirty}><Card bordered={false}><NewCoinGrant project={p} onSaved={reload}/></Card></TabPane> : null}
    </Tabs>
  </main>;
}
