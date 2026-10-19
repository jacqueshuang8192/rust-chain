import { Button, Modal } from '@douyinfe/semi-ui';
import { useState } from 'react';
import type { ApiRecord } from '../../api/types';
import { IssuancePolicySummary } from '../../coin-applications/IssuancePolicySummary';
import { lockLabel, validatePolicy, type IssuancePolicy } from '../../coin-applications/issuancePolicy';
import type { ResourceConfig } from '../resources/resourceConfigs';
import { NewCoinResourcePage } from './NewCoinResourcePage';
import '../../coin-applications/coinApplications.css';

function policyOf(row: ApiRecord): IssuancePolicy | null {
  try { validatePolicy(row.policy_json); return row.policy_json; } catch { return null; }
}
function HoldingRules({ row }: { row: ApiRecord }) {
  const [open, setOpen] = useState(false);
  const policy = policyOf(row);
  return <><Button size="small" onClick={() => setOpen(true)} aria-label={`查看持仓 ${row.id} 原始规则`}>原始规则</Button>
    <Modal title={`持仓 ${row.id} · 原始锁仓与费用`} visible={open} footer={null} onCancel={() => setOpen(false)} width="min(560px, calc(100vw - 32px))">
      {policy ? <IssuancePolicySummary policy={policy} quote={String(row.quote_symbol ?? '')} /> : <p role="alert">持仓合同不完整，请核对记录。</p>}
      <p>规则以购买时保存的合同为准；解禁到期只允许用户确认回购，不会自动卖出。</p>
    </Modal></>;
}
const filters: NonNullable<ResourceConfig['filters']> = [
  { key: 'project_id', label: '项目ID' }, { key: 'user_id', label: '用户ID' }, { key: 'email', label: '邮箱' },
  { key: 'agent_id', label: '买家直属代理ID' }, { key: 'issuer_agent_id', label: '发行代理ID' }, { key: 'limit', label: '每页数量' },
];
const identities: ResourceConfig['columns'] = [
  { key: 'symbol', title: '项目币种' }, { key: 'project_id', title: '项目ID' }, { key: 'email', title: '用户邮箱' }, { key: 'user_id', title: '用户ID' },
  { key: 'owner_agent_code', title: '买家直属代理' }, { key: 'owner_agent_id', title: '买家直属代理ID' },
  { key: 'issuer_agent_id', title: '发行代理ID', render: row => row.issuer_agent_id == null ? '平台直接发行' : String(row.issuer_agent_id) },
];
export const tokenRecordConfigs: Record<'holdings' | 'buybacks' | 'operations', ResourceConfig> = {
  holdings: {
    title: '持仓与锁仓规则', endpoint: '/admin/api/v1/new-coins/holdings', responseKey: 'holdings',
    csvFileName: 'token-holdings.csv', notice: '新版发行持仓保留购买时的锁仓与费用规则。到期不自动卖出，实际回购结果见解锁回购记录。',
    filters: [...filters, { key: 'status', label: '持仓状态', type: 'select', options: [{ value: 'held', label: '持有中' }, { value: 'sold', label: '已回购卖出' }] }],
    rowActions: (row, helpers) => <><HoldingRules row={row} /><Button size="small" onClick={() => helpers.openDetail({ title: `持仓 ${row.id}`, data: row })}>查看详情</Button></>,
    columns: [
      { key: 'id', title: '持仓ID' }, ...identities, { key: 'source_kind', title: '取得方式', valueMap: { subscription: '申购获配', purchase: '实时认购' } },
      { key: 'source_id', title: '来源订单ID' }, { key: 'quantity', title: '持仓数量', type: 'amount' }, { key: 'cost', title: '原始成本', type: 'amount' }, { key: 'quote_symbol', title: '成本计价币种' },
      { key: 'policy_json', title: '锁仓规则', type: 'json', render: row => { const p = policyOf(row); return p ? lockLabel(p.lock) : '规则不可用'; } },
      { key: 'acquired_at', title: '原购买时间', type: 'timestamp' }, { key: 'unlock_at', title: '有效解禁时间', type: 'timestamp' },
      { key: 'status', title: '持仓状态', valueMap: { held: '持有中', sold: '已回购卖出' } }, { key: 'sold_at', title: '卖出时间', type: 'timestamp' },
    ],
  },
  buybacks: {
    title: '解锁回购记录', endpoint: '/admin/api/v1/new-coins/buybacks', responseKey: 'buybacks', filters,
    csvFileName: 'token-buybacks.csv', notice: '仅展示已完成的整批回购，手续费已从卖出总额扣除；未成交报价不计入到账。',
    rowActions: (row, helpers) => <Button size="small" onClick={() => helpers.openDetail({ title: `回购成交 ${row.id}`, data: row,
      fieldMeta: { labels: { source: '行情来源', source_version: '行情版本' }, valueMaps: { source: { default: '默认行情', strategy: '策略行情' } } },
    })}>查看详情</Button>,
    columns: [
      { key: 'id', title: '成交凭证' }, ...identities, { key: 'holding_id', title: '持仓ID' }, { key: 'quantity', title: '卖出数量', type: 'amount' },
      { key: 'price', title: '成交单价', type: 'amount' }, { key: 'cost', title: '原始成本', type: 'amount' },
      { key: 'gross', title: '卖出总额', type: 'amount' }, { key: 'fee', title: '扣除手续费', type: 'amount' }, { key: 'net', title: '实际到账', type: 'amount' },
      { key: 'quote_symbol', title: '结算币种' }, { key: 'status', title: '状态', valueMap: { executed: '已结算' } },
      { key: 'observed_at', title: '行情采样时间', type: 'timestamp' }, { key: 'executed_at', title: '成交时间', type: 'timestamp' },
    ],
  },
  operations: {
    title: '发行操作记录', endpoint: '/admin/api/v1/new-coins/operations', responseKey: 'operations',
    csvFileName: 'token-operations.csv',
    filters: [{ key: 'project_id', label: '项目ID' }, { key: 'actor_type', label: '执行角色', type: 'select', options: [{ value: 'admin', label: '管理员' }, { value: 'agent', label: '代理' }, { value: 'scheduler', label: '自动调度' }, { value: 'user', label: '用户' }] }, { key: 'actor_id', label: '执行者ID' },
      { key: 'action', label: '操作类型', type: 'select', options: [{ value: 'new_coin.stage', label: '阶段推进' }, { value: 'new_coin.allocate', label: '派发与退款' }, { value: 'new_coin.trade', label: '认购与回购成交' }, { value: 'new_coin.issuance.create', label: '管理员直接发行' }] }, { key: 'limit', label: '每页数量' }],
    columns: [{ key: 'id', title: '记录ID' }, { key: 'project_id', title: '项目ID' }, { key: 'symbol', title: '项目币种' },
      { key: 'actor_type', title: '执行角色', valueMap: { admin: '管理员', agent: '代理', scheduler: '自动调度', user: '用户' } }, { key: 'actor_id', title: '执行者ID' },
      { key: 'action', title: '操作', valueMap: { 'new_coin.stage': '阶段推进', 'new_coin.allocate': '派发与退款', 'new_coin.trade': '认购与回购成交', 'new_coin.issuance.create': '管理员直接发行' } },
      { key: 'reason', title: '操作原因' }, { key: 'payload_json', title: '操作详情', type: 'json' }, { key: 'created_at', title: '操作时间', type: 'timestamp' }],
  },
};
export function TokenHoldingsPage() { return <NewCoinResourcePage config={tokenRecordConfigs.holdings} />; }
export function TokenBuybacksPage() { return <NewCoinResourcePage config={tokenRecordConfigs.buybacks} />; }
export function TokenOperationsPage() { return <NewCoinResourcePage config={tokenRecordConfigs.operations} />; }
