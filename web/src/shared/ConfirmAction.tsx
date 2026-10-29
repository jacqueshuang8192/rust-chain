import { adminErrorMessage } from './adminErrorMessage';
import { Button, Modal, TextArea, Typography } from '@douyinfe/semi-ui';
import { useState } from 'react';

const { Text } = Typography;

type ConfirmActionProps = {
  actionAriaLabel?: string;
  actionText?: string;
  confirmText?: string;
  dangerous?: boolean;
  disabled?: boolean;
  description?: string;
  modalWidth?: number | string;
  modalClassName?: string;
  onConfirm: (reason: string) => Promise<void> | void;
  /** 通知父页面暂停会改变待确认目标的后台刷新。 */
  onOpenChange?: (open: boolean) => void;
  /** 允许不填原因直接确认；用于不强制审计说明的操作，例如秒合约控输控赢。 */
  reasonOptional?: boolean;
  title: string;
};

export function ConfirmAction({ actionAriaLabel, actionText = '执行', confirmText = '确认', dangerous, description, disabled, modalWidth, modalClassName, onConfirm, onOpenChange, reasonOptional = false, title }: ConfirmActionProps) {
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const [visible, setVisible] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const destructive = dangerous ?? /删除|拒绝|驳回|作废|禁用|停用|标记失败|注销|移除|冲正|回收|撤单|重置|归档/.test(`${actionText}${title}`);

  function closeModal() {
    if (submitting) {
      return;
    }
    setVisible(false);
    onOpenChange?.(false);
    setReason('');
    setError('');
  }

  async function handleConfirm() {
    const trimmed = reason.trim();
    if (!trimmed && !reasonOptional) {
      return;
    }

    setSubmitting(true);
    setError('');
    try {
      await onConfirm(trimmed);
      setVisible(false);
      onOpenChange?.(false);
      setReason('');
    } catch (cause) {
      // Keep the reason and dialog on a rejected action; do not leak an unhandled Modal promise.
      setError(adminErrorMessage(cause, '操作失败，请核对状态后重试'));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <Button aria-label={actionAriaLabel} disabled={disabled} onClick={() => { setVisible(true); onOpenChange?.(true); }} theme={destructive ? 'light' : 'solid'} type={destructive ? 'danger' : 'primary'}>
        {actionText}
      </Button>
      <Modal
        className={modalClassName}
        cancelButtonProps={{ 'aria-label': '取消', disabled: submitting }}
        closeOnEsc={!submitting}
        confirmLoading={submitting}
        maskClosable={false}
        motion={false}
        okButtonProps={{ 'aria-label': confirmText, disabled: !reasonOptional && reason.trim().length === 0, type: destructive ? 'danger' : 'primary' }}
        okText={confirmText}
        onCancel={closeModal}
        onOk={handleConfirm}
        title={title}
        visible={visible}
        width={modalWidth}
      >
        {description ? <p>{description}</p> : null}
        <Text type="secondary">{reasonOptional ? '原因可选填，留空只在审计中记录操作人与时间。' : '请输入非空原因后继续。'}</Text>
        {error ? <div role="alert"><Text type="danger">{error}</Text></div> : null}
        <TextArea
          aria-label="操作原因"
          autosize
          onChange={setReason}
          placeholder={reasonOptional ? '操作原因（可选）' : '请输入操作原因'}
          style={{ marginTop: 12 }}
          value={reason}
        />
      </Modal>
    </>
  );
}
