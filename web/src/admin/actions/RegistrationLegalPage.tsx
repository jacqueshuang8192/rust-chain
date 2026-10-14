import { IconRefresh, IconSave, IconSearch } from '@douyinfe/semi-icons';
import { Button, Modal, Spin, Typography } from '@douyinfe/semi-ui';
import { createElement, useEffect, useRef, useState, type ReactNode } from 'react';
import { apiRequest, ApiError } from '../../api/client';
import { PageHeader } from '../../layouts/PageHeader';
import { QuillRichTextEditor, type RichTextLeaf, type RichTextValue } from '../../shared/QuillRichTextEditor';
import { AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { useCanAdminRequest } from '../access';
import { UnsavedChangesGuard } from '../settings/UnsavedChangesGuard';
import './RegistrationLegalPage.css';

type Document = { id: number; title: string; body: string | RichTextValue; effective_on: string };
type Bundle = { publication_id: number; scope_code: string; locale: string; edition: number; terms: Document; privacy: Document };
type Edition = { id: number; edition: number; published_at: string; published_by: number };
type LegalState = { current: Bundle | null; editions: Edition[] };
type Country = { country_code: string; country_name: string; supported_locales: string[] };
type Receipt = { user_id: number; country_code: string; scope_code: string; locale: string; publication_id: number; terms_id: number; privacy_id: number; acknowledged_at: string };
type Draft = { termsTitle: string; termsBody: RichTextValue; privacyTitle: string; privacyBody: RichTextValue; effectiveOn: string; reason: string };

const path = '/admin/api/v1/legal-documents';
const emptyBody = (): RichTextValue => [{ type: 'p', children: [{ text: '' }] }];
const toRichText = (body: string | RichTextValue): RichTextValue => typeof body === 'string'
  ? body.replace(/\r\n/g, '\n').split('\n').map((text) => ({ type: 'p', children: [{ text }] }))
  : body;
const emptyDraft = (): Draft => ({ termsTitle: '', termsBody: emptyBody(), privacyTitle: '', privacyBody: emptyBody(), effectiveOn: new Date().toISOString().slice(0, 10), reason: '' });
const fromCurrent = (current: Bundle | null): Draft => current ? {
  termsTitle: current.terms.title, termsBody: toRichText(current.terms.body),
  privacyTitle: current.privacy.title, privacyBody: toRichText(current.privacy.body),
  effectiveOn: current.terms.effective_on, reason: ''
} : emptyDraft();

function bodyIssue(body: RichTextValue, label: string): string | null {
  if (body.some((block) => block.type === 'image')) return `${label}不支持图片，请删除后发布。`;
  if (!body.some((block) => block.type !== 'image' && block.children.some((leaf) => leaf.text.trim()))) return `请填写${label}正文。`;
  if (new TextEncoder().encode(JSON.stringify(body)).length > 131_072) return `${label}正文不能超过128 KiB。`;
  return null;
}

function legalLeaf(leaf: RichTextLeaf, index: number): ReactNode {
  let content: ReactNode = leaf.text;
  if (leaf.bold) content = <strong>{content}</strong>;
  if (leaf.italic) content = <em>{content}</em>;
  if (leaf.underline) content = <u>{content}</u>;
  return <span key={index}>{content}</span>;
}

function LegalBodyView({ body }: { body: string | RichTextValue }) {
  const blocks = toRichText(body);
  return <div className="legal-editor-body-preview">{blocks.map((block, index) => block.type === 'image' ? null
    : createElement(block.type, { key: index }, block.children.map(legalLeaf)))}</div>;
}

function legalQuery(scope: string, locale: string) {
  return `${path}?scope_code=${encodeURIComponent(scope)}&locale=${encodeURIComponent(locale)}`;
}

export function RegistrationLegalPage() {
  const canWrite = useCanAdminRequest(path, 'POST');
  const [countries, setCountries] = useState<Country[]>([]);
  const [scope, setScope] = useState('GLOBAL');
  const [locale, setLocale] = useState('en');
  const [snapshot, setSnapshot] = useState<LegalState | null>(null);
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [selectedHistory, setSelectedHistory] = useState<Bundle | null>(null);
  const [receiptUser, setReceiptUser] = useState('');
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [receiptError, setReceiptError] = useState('');
  const requestId = useRef(0);
  const dirty = Boolean(snapshot && JSON.stringify({ ...draft, reason: '' }) !== JSON.stringify({ ...fromCurrent(snapshot.current), reason: '' })) || Boolean(draft.reason);

  useEffect(() => {
    void apiRequest<{ countries: Country[] }>('/api/v1/countries', { auth: 'none' })
      .then((data) => setCountries(data.countries.filter((country) => /^[A-Z]{2}$/.test(country.country_code))))
      .catch(() => setCountries([]));
  }, []);

  async function load(nextScope = scope, nextLocale = locale) {
    const id = ++requestId.current;
    setLoading(true);
    setError('');
    try {
      const data = await apiRequest<LegalState>(legalQuery(nextScope, nextLocale));
      if (id !== requestId.current) return;
      if (!Array.isArray(data.editions) || (data.current && !Number.isSafeInteger(data.current.publication_id))) {
        throw new Error('法律文件响应格式错误');
      }
      setSnapshot(data);
      setDraft(fromCurrent(data.current));
    } catch (reason) {
      if (id === requestId.current) setError(reason instanceof Error ? reason.message : '加载法律文件失败');
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }

  useEffect(() => { void load(scope, locale); return () => { requestId.current += 1; }; }, [scope, locale]);

  function changeScope(next: string) {
    if (dirty && !window.confirm('当前编辑内容尚未发布，切换地区会放弃修改。')) return;
    setScope(next);
    setSnapshot(null);
  }

  function changeLocale(next: string) {
    if (dirty && !window.confirm('当前编辑内容尚未发布，切换语言会放弃修改。')) return;
    setLocale(next);
    setSnapshot(null);
  }

  function change<K extends keyof Draft>(field: K, value: Draft[K]) {
    setDraft((current) => ({ ...current, [field]: value }));
  }

  const validationIssues = [
    !draft.termsTitle.trim() ? '请填写用户协议标题。' : [...draft.termsTitle].length > 160 ? '用户协议标题不能超过160字。' : null,
    bodyIssue(draft.termsBody, '用户协议'),
    !draft.privacyTitle.trim() ? '请填写隐私政策标题。' : [...draft.privacyTitle].length > 160 ? '隐私政策标题不能超过160字。' : null,
    bodyIssue(draft.privacyBody, '隐私政策'),
    !/^\d{4}-\d{2}-\d{2}$/.test(draft.effectiveOn) ? '请选择生效日期。' : draft.effectiveOn > new Date().toISOString().slice(0, 10) ? '生效日期不能晚于UTC当天。' : null,
    !draft.reason.trim() ? '请填写发布原因。' : [...draft.reason].length > 500 ? '发布原因不能超过500字。' : null
  ].filter((issue): issue is string => Boolean(issue));
  const valid = validationIssues.length === 0;

  async function publish() {
    if (!snapshot || !valid || saving) return;
    setSaving(true);
    setError('');
    try {
      await apiRequest(path, { method: 'POST', body: JSON.stringify({
        scope_code: scope, locale, expected_edition: snapshot.current?.edition ?? 0,
        effective_on: draft.effectiveOn, terms_title: draft.termsTitle,
        terms_body: draft.termsBody, privacy_title: draft.privacyTitle,
        privacy_body: draft.privacyBody, reason: draft.reason
      }) });
      setConfirmOpen(false);
      setMessage('两份文件已同时发布。');
      await load();
    } catch (reason) {
      setError(reason instanceof ApiError && reason.status === 409
        ? '版本已更新，请刷新后重新核对正文。'
        : reason instanceof Error ? reason.message : '发布失败');
    } finally {
      setSaving(false);
    }
  }

  async function showHistory(id: number) {
    try { setSelectedHistory(await apiRequest<Bundle>(`${path}/publications/${id}`)); }
    catch (reason) { setError(reason instanceof Error ? reason.message : '加载历史版本失败'); }
  }

  async function findReceipt() {
    if (!/^[1-9]\d*$/.test(receiptUser) || !Number.isSafeInteger(Number(receiptUser))) {
      setReceiptError('请输入有效的用户 ID。'); return;
    }
    setReceiptError(''); setReceipt(null);
    try { setReceipt(await apiRequest<Receipt>(`${path}/receipts/${receiptUser}`)); }
    catch (reason) { setReceiptError(reason instanceof Error ? reason.message : '未找到确认记录'); }
  }

  const localeOptions = [...new Set(['en', 'zh-CN', ...(countries.find((country) => country.country_code === scope)?.supported_locales ?? [])])]
    .map((value) => ({ value, label: value }));

  return <main className="exchange-page admin-action-page legal-editor-page">
    <UnsavedChangesGuard enabled={dirty} />
    <PageHeader title="用户协议与隐私政策" actions={<Button icon={<IconRefresh aria-hidden />} loading={loading} onClick={() => void load()}>刷新</Button>} />
    <div className="legal-editor-toolbar">
      <label>适用地区<AdminSelect ariaLabel="适用地区" value={scope} onChange={changeScope}
        optionList={[{ value: 'GLOBAL', label: '统一版本' }, ...countries.map((country) => ({ value: country.country_code, label: `${country.country_name} (${country.country_code})` }))]} /></label>
      <label>语言<AdminSelect ariaLabel="语言" value={locale} onChange={changeLocale} optionList={localeOptions} /></label>
      <span className="legal-editor-edition">{snapshot?.current ? `已发布第 ${snapshot.current.edition} 版` : '尚未发布'}</span>
    </div>
    {loading && !snapshot && <Spin aria-label="正在加载法律文件" />}
    {error && <p role="alert" className="legal-editor-error">{error}</p>}
    {message && <p role="status" className="legal-editor-success">{message}</p>}
    {snapshot && <>
      <div className="legal-editor-fields">
        <label>生效日期<AdminTextInput ariaLabel="生效日期" type="date" disabled={!canWrite} value={draft.effectiveOn} onChange={(value) => change('effectiveOn', value)} /></label>
        <label>发布原因<AdminTextInput ariaLabel="发布原因" disabled={!canWrite} value={draft.reason} onChange={(value) => change('reason', value)} /></label>
      </div>
      <div className="legal-editor-documents">
        <section><h2>用户协议</h2>
          <label>标题<AdminTextInput ariaLabel="用户协议标题" disabled={!canWrite} value={draft.termsTitle} onChange={(value) => change('termsTitle', value)} /></label>
          <div className="legal-editor-body-field"><span>正文</span>{canWrite
            ? <QuillRichTextEditor ariaLabel="用户协议正文" placeholder="请输入用户协议正文" value={draft.termsBody} onChange={(value) => change('termsBody', value)} />
            : <LegalBodyView body={draft.termsBody} />}</div>
        </section>
        <section><h2>隐私政策</h2>
          <label>标题<AdminTextInput ariaLabel="隐私政策标题" disabled={!canWrite} value={draft.privacyTitle} onChange={(value) => change('privacyTitle', value)} /></label>
          <div className="legal-editor-body-field"><span>正文</span>{canWrite
            ? <QuillRichTextEditor ariaLabel="隐私政策正文" placeholder="请输入隐私政策正文" value={draft.privacyBody} onChange={(value) => change('privacyBody', value)} />
            : <LegalBodyView body={draft.privacyBody} />}</div>
        </section>
      </div>
      {canWrite && <div className="legal-editor-commands">
        {validationIssues.length > 0 && <ul className="legal-editor-validation" aria-label="发布前需完成">{validationIssues.map((issue) => <li key={issue}>{issue}</li>)}</ul>}
        <Button icon={<IconSave aria-hidden />} disabled={!valid || loading || saving} loading={saving} theme="solid" onClick={() => setConfirmOpen(true)}>发布两份文件</Button>
      </div>}
      <section className="legal-editor-history"><h2>发布历史</h2>
        {snapshot.editions.length ? <div className="legal-editor-history-list">{snapshot.editions.map((edition) =>
          <button type="button" key={edition.id} onClick={() => void showHistory(edition.id)}>第 {edition.edition} 版 · {new Date(edition.published_at).toLocaleString()} · 管理员 {edition.published_by}</button>)}</div> : <p>暂无发布记录</p>}
      </section>
      <section className="legal-editor-receipts"><h2>用户确认记录</h2>
        <div className="legal-editor-receipt-search"><AdminTextInput ariaLabel="用户 ID" value={receiptUser} onChange={setReceiptUser} /><Button icon={<IconSearch aria-hidden />} onClick={() => void findReceipt()}>查询</Button></div>
        {receiptError && <p role="alert">{receiptError}</p>}
        {receipt && <dl><dt>国家 / 语言</dt><dd>{receipt.country_code} · {receipt.locale}</dd><dt>确认时间</dt><dd>{new Date(receipt.acknowledged_at).toLocaleString()}</dd><dt>发布版本</dt><dd>{receipt.publication_id}</dd><dt>文件编号</dt><dd>用户协议 {receipt.terms_id} · 隐私政策 {receipt.privacy_id}</dd></dl>}
      </section>
    </>}
    <Modal visible={confirmOpen} title="确认发布用户协议与隐私政策" okText="发布" confirmLoading={saving}
      okButtonProps={{ 'aria-label': '发布法律文件' }} cancelButtonProps={{ 'aria-label': '取消发布' }}
      onCancel={() => setConfirmOpen(false)} onOk={() => void publish()}>
      <Typography.Text>发布后，适用地区的新注册用户需要确认这个版本。原有注册确认记录不会改写。</Typography.Text>
    </Modal>
    <Modal visible={Boolean(selectedHistory)} title={selectedHistory ? `第 ${selectedHistory.edition} 版 · ${selectedHistory.scope_code} · ${selectedHistory.locale}` : '历史版本'} footer={null} onCancel={() => setSelectedHistory(null)}>
      {selectedHistory && <div className="legal-editor-history-copy"><h3>{selectedHistory.terms.title}</h3><LegalBodyView body={selectedHistory.terms.body} /><h3>{selectedHistory.privacy.title}</h3><LegalBodyView body={selectedHistory.privacy.body} /></div>}
    </Modal>
  </main>;
}
