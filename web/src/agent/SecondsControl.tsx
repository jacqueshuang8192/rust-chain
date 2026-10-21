import { Banner, Button, Modal, Space, Tag, TextArea, Typography } from '@douyinfe/semi-ui';
import { useEffect, useState } from 'react';

import {
  getAgentSecondsUserControl,
  setAgentSecondsOrderControl,
  setAgentSecondsUserControl,
  type AgentSecondsControlResult,
  type AgentSecondsUserControlResponse
} from '../api/agent';
import { adminErrorMessage } from '../shared/adminErrorMessage';

const { Text } = Typography;

/** 控制结果的中文短标签，用于按钮与状态标签。 */
function controlLabel(result: AgentSecondsControlResult): string {
  if (result === 'win') return '控赢';
  if (result === 'loss') return '控输';
  return '取消控制';
}

/** 把后端回显的控制结果翻译成列表可见的状态文案。 */
function statusLabel(result: 'win' | 'loss' | null | undefined): string | null {
  if (result === 'win') return '已控赢';
  if (result === 'loss') return '已控输';
  return null;
}

type ControlModalProps = {
  current: 'win' | 'loss' | null;
  description?: string;
  onCancel: () => void;
  onSubmit: (result: AgentSecondsControlResult, reason: string) => Promise<void>;
  title: string;
  visible: boolean;
};

/**
 * 控输控赢确认弹窗：选择目标结果，原因可选；提交失败时保留弹窗与已填内容。
 * 已在生效中的控制允许一键取消；取消同样需要原因，便于控制流水追溯操作人。
 */
function ControlModal({ current, description, onCancel, onSubmit, title, visible }: ControlModalProps) {
  const [selected, setSelected] = useState<AgentSecondsControlResult>(current ?? 'win');
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (visible) {
      setSelected(current ?? 'win');
      setReason('');
      setError('');
    }
  }, [current, visible]);

  async function confirm() {
    const trimmed = reason.trim();
    setSubmitting(true);
    setError('');
    try {
      await onSubmit(selected, trimmed);
    } catch (cause) {
      setError(adminErrorMessage(cause, '控制设置失败，请核对订单状态后重试'));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      cancelButtonProps={{ 'aria-label': '取消', disabled: submitting }}
      closeOnEsc={!submitting}
      confirmLoading={submitting}
      maskClosable={false}
      motion={false}
      okButtonProps={{ 'aria-label': '确认控制', type: selected === 'clear' ? 'tertiary' : 'primary' }}
      okText="确认控制"
      onCancel={() => { if (!submitting) onCancel(); }}
      onOk={confirm}
      title={title}
      visible={visible}
      width={520}
    >
      {description ? <p>{description}</p> : null}
      <Space wrap>
        {(['win', 'loss'] as const).map((value) => (
          <Button
            aria-label={controlLabel(value)}
            disabled={submitting}
            key={value}
            onClick={() => setSelected(value)}
            theme={selected === value ? 'solid' : 'light'}
            type={value === 'win' ? 'primary' : 'danger'}
          >
            {controlLabel(value)}
          </Button>
        ))}
        <Button
          aria-label="取消控制"
          disabled={submitting || current === null}
          onClick={() => setSelected('clear')}
          theme={selected === 'clear' ? 'solid' : 'light'}
        >
          取消控制
        </Button>
      </Space>
      <p><Text type="secondary">控赢/控输决定该单最终派奖结果，取消控制后按真实行情判定；原因可选填。</Text></p>
      {error ? <Banner description={error} type="danger" /> : null}
      <TextArea
        aria-label="操作原因"
        autosize
        disabled={submitting}
        onChange={setReason}
        placeholder="操作原因（可选）"
        style={{ marginTop: 12 }}
        value={reason}
      />
    </Modal>
  );
}

type SecondsControlActionsProps = {
  current: 'win' | 'loss' | null;
  disabled?: boolean;
  onDone: () => void;
  orderId: number;
};

/**
 * 代理订单列表里的单笔控制入口：按钮直接展示当前控制状态，点击后打开确认弹窗。
 * 订单已结算或正在结算时后端会返回冲突，这里只负责把错误原样交给弹窗展示。
 */
export function SecondsControlActions({ current, disabled, onDone, orderId }: SecondsControlActionsProps) {
  const [visible, setVisible] = useState(false);
  const label = statusLabel(current) ?? '控输控赢';
  return (
    <>
      <Button
        aria-label={`控输控赢 订单 ${orderId}`}
        disabled={disabled}
        onClick={() => setVisible(true)}
        theme="borderless"
        type={current === 'loss' ? 'danger' : 'primary'}
      >
        {label}
      </Button>
      <ControlModal
        current={current}
        description={`订单 ${orderId}：控制只在订单结算前生效。`}
        onCancel={() => setVisible(false)}
        onSubmit={async (result, reason) => {
          await setAgentSecondsOrderControl(orderId, result, reason);
          setVisible(false);
          onDone();
        }}
        title="设置订单控输控赢"
        visible={visible}
      />
    </>
  );
}

type UserSecondsControlPanelProps = {
  onChanged: () => void;
  userId: number;
};

/**
 * 用户级控制面板：展示该用户当前默认控制，并提供控赢/控输/取消入口。
 * 用户级设置只影响之后开仓的订单，已开仓订单仍需在订单列表逐单控制。
 */
export function UserSecondsControlPanel({ onChanged, userId }: UserSecondsControlPanelProps) {
  const [state, setState] = useState<{ data: AgentSecondsUserControlResponse | null; error: Error | null; loading: boolean }>({
    data: null,
    error: null,
    loading: true
  });
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    let active = true;
    setState({ data: null, error: null, loading: true });
    getAgentSecondsUserControl(userId)
      .then((data) => { if (active) setState({ data, error: null, loading: false }); })
      .catch((error: unknown) => { if (active) setState({ data: null, error: error instanceof Error ? error : new Error('加载失败'), loading: false }); });
    return () => { active = false; };
  }, [userId]);

  const current = state.data?.control_result ?? null;

  return (
    <div className="agent-portfolio-filters" aria-label="秒合约用户级控制">
      <Space wrap>
        <Text strong>默认控输控赢</Text>
        {state.loading ? <Text type="tertiary">加载中…</Text> : null}
        {state.error ? <Text type="danger">控制状态加载失败</Text> : null}
        {state.data ? (
          current
            ? <Tag color={current === 'win' ? 'green' : 'red'}>{statusLabel(current)}</Tag>
            : <Tag>未设置</Tag>
        ) : null}
        {state.data?.reason ? <Text type="tertiary">原因：{state.data.reason}</Text> : null}
        <Button
          aria-label="设置用户级控输控赢"
          disabled={state.loading || Boolean(state.error)}
          onClick={() => setVisible(true)}
        >
          设置默认控制
        </Button>
      </Space>
      <ControlModal
        current={current}
        description={`用户 ${userId}：默认控制对该用户之后开仓的秒合约订单生效。`}
        onCancel={() => setVisible(false)}
        onSubmit={async (result, reason) => {
          const response = await setAgentSecondsUserControl(userId, result, reason);
          setVisible(false);
          setState({ data: { ...(state.data ?? { user_id: userId, active: false, control_result: null, actor_type: null, actor_id: null, reason: null, control_at: null }), active: result !== 'clear', control_result: response.control_result, actor_type: response.actor_type, actor_id: response.actor_id, reason: response.reason, control_at: response.control_at }, error: null, loading: false });
          onChanged();
        }}
        title="设置用户级控输控赢"
        visible={visible}
      />
    </div>
  );
}
