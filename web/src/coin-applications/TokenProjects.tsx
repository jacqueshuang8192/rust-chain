import { Button, Banner } from '@douyinfe/semi-ui';
import { useEffect, useState } from 'react';
import { tokenProjects, advanceTokenProject, stageLabels, nextStages, type TokenProject } from '../api/tokenIssuance';
import { authStore, type AuthSession } from '../auth/authStore';
import { ConfirmAction } from '../shared/ConfirmAction';
import { TimestampText } from '../shared/TimestampText';
import { adminErrorMessage } from '../shared/adminErrorMessage';
import { IssuancePolicySummary } from './IssuancePolicySummary';
export function TokenProjects({ session, revision }: { session: AuthSession; revision: number }) {
  const [rows, setRows] = useState<TokenProject[]>([]), [error, setError] = useState(''), [loading, setLoading] = useState(true), [reload, setReload] = useState(0);
  const [before, setBefore] = useState<number>(), [next, setNext] = useState<number | null>(null);
  const [busy, setBusy] = useState<number | null>(null), [uncertain, setUncertain] = useState(false);
  useEffect(() => { const c = new AbortController(); setLoading(true);
    void tokenProjects(c.signal, before).then(v => { if (!c.signal.aborted) { setRows(v.projects); setNext(v.next); setError(''); setUncertain(false); } }).catch(e => { if (!c.signal.aborted) setError(adminErrorMessage(e)); }).finally(() => { if (!c.signal.aborted) setLoading(false); });
    return () => c.abort();
  }, [revision, reload, before]);
  return <section className="token-projects"><p>管理审核通过的发行项目。自动节点按计划推进，手动节点需确认；派发在下级申购列表中分批完成。</p>
    <Button loading={loading} disabled={busy !== null} onClick={() => setReload(v => v + 1)}>刷新发行项目</Button>
    {error ? <Banner type="danger" description={error} /> : null}
    {!loading && !rows.length && !error ? <p>暂无已通过审核的发行项目。</p> : null}
    {rows.map(p => <article className="token-project-row" key={p.id}><h3>{p.symbol} · {stageLabels[p.lifecycle_status]}</h3>
      {p.transition_error ? <Banner type="warning" description="自动推进暂未完成，请核对待派发订单与行情状态，系统会继续重试。" /> : null}
      {p.next_transition_at !== null ? <p>下一自动节点：<TimestampText value={p.next_transition_at} /></p> : null}
      <details><summary>查看发行、时间和锁仓规则</summary><IssuancePolicySummary policy={p.issuance_policy} /></details>
      {nextStages[p.lifecycle_status] ? <ConfirmAction actionText={`进入${stageLabels[nextStages[p.lifecycle_status]]}`} title={`推进 ${p.symbol} 的阶段`} disabled={busy !== null || uncertain || loading}
        description={p.lifecycle_status === 'distribution' ? '全部申购需先完成派发或退款；进入认购后将启动平台实时行情。' : p.lifecycle_status === 'purchase' ? '结束认购并转入现货，保留行情与历史 K 线；已有持仓继续原锁仓规则。' : '仅推进项目阶段，不自动派发或卖出用户资产。'}
        onConfirm={async reason => { if (authStore.getSession('agent')?.generation !== session.generation) throw new Error('会话已变化'); setBusy(p.id);
          try { const row = await advanceTokenProject(p, reason); setRows(list => list.map(v => v.id === row.id ? row : v)); setError(''); }
          catch (e) { setUncertain(true); setError('操作结果待核对，请刷新发行项目后继续。'); throw e; } finally { setBusy(null); }
        }} /> : <p>已转入现货；已有持仓可继续在用户端查看并按规则解锁卖出。</p>}
    </article>)}
    <nav aria-label="发行项目分页">
      {before ? <Button disabled={loading || busy !== null} onClick={() => setBefore(undefined)}>最新项目</Button> : null}
      {next ? <Button disabled={loading || busy !== null} onClick={() => setBefore(next)}>更早项目</Button> : null}
    </nav>
  </section>;
}
