import { adminErrorFieldValue } from './adminErrorMessage';
import { Empty, SideSheet } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';

import { adminFieldLabel, adminEnumLabel } from './adminPresentation';
import type { ApiRecord } from '../api/types';
import { formatAdminBetContent, isAdminBetContentField } from './betContentFormat';
import { formatAdminDisplayValue, formatAdminNumber } from './numberFormat';
import { formatBusinessOrderNo } from './orderNo';
import { ResizableTable } from './ResizableTable';
import { containedTableStyle } from './tableLayout';
import { formatAdminTimestamp } from './TimestampText';
import './DetailDrawer.css';

type DetailFieldType = 'amount' | 'json' | 'status' | 'text' | 'timestamp';

type DetailDrawerFieldMeta = {
  assets?: Record<string, string | undefined>;
  labels?: Record<string, string>;
  types?: Record<string, DetailFieldType | undefined>;
  valueMaps?: Record<string, Record<string, string> | undefined>;
};

type DetailDrawerData = {
  data: ApiRecord | ApiRecord[];
  fieldMeta?: DetailDrawerFieldMeta;
  title?: string;
};

type DetailDrawerProps = {
  detail: DetailDrawerData | null;
  onClose: () => void;
};

function fieldLabel(key: string, meta?: DetailDrawerFieldMeta) {
  const label = meta?.labels?.[key];
  return typeof label === 'string' ? label : adminFieldLabel(key);
}

function mappedValue(value: unknown, key: string, meta?: DetailDrawerFieldMeta): string | null {
  if (value === null || value === undefined || value === '') {
    return null;
  }
  const label = meta?.valueMaps?.[key]?.[String(value)];
  return typeof label === 'string' ? label : adminEnumLabel(key, value, meta?.types?.[key] === 'status');
}

function businessOrderFieldValue(value: unknown, key: string): string | null {
  const prefixes: Record<string, string> = {
    buy_order_id: 'SP',
    sell_order_id: 'SP',
    subscription_id: 'NC'
  };
  const prefix = prefixes[key];
  if (typeof prefix !== 'string' || value === null || value === undefined || value === '') {
    return null;
  }
  return formatBusinessOrderNo(prefix, { id: value });
}

function typedDisplayValue(value: unknown, key: string, meta?: DetailDrawerFieldMeta): string | null {
  const orderNoValue = businessOrderFieldValue(value, key);
  if (orderNoValue) {
    return orderNoValue;
  }

  const type = meta?.types?.[key];
  if ((type === 'timestamp' || (type === undefined && adminFieldLabel(key) !== key && /(?:_at|_time)$/.test(key))) && typeof value === 'number') {
    return formatAdminTimestamp(value);
  }
  if (type === 'amount') {
    const formatted = typeof value === 'string' || typeof value === 'number' ? formatAdminNumber(value) : null;
    return formatted ? `${formatted}${meta?.assets?.[key] ? ` ${meta.assets[key]}` : ''}` : null;
  }
  if (type === 'status') {
    return mappedValue(value, key, meta);
  }
  return null;
}

export function displayDetailValue(value: unknown, key = '', meta?: DetailDrawerFieldMeta): string {
  if (value === null || value === undefined || value === '') {
    return '-';
  }

  if (isAdminBetContentField(key, fieldLabel(key, meta))) {
    const formattedBetContent = formatAdminBetContent(value);
    if (formattedBetContent) {
      return formattedBetContent;
    }
  }

  if (Array.isArray(value)) {
    return value.map((item) => displayDetailValue(item, key, meta)).join(' / ');
  }

  if (typeof value === 'object') {
    const nestedMeta = key === 'nodes' ? { ...meta, labels: { ...meta?.labels, target_type: '目标类型' } } : meta;
    return Object.entries(value as ApiRecord)
      .map(([itemKey, item]) => `${fieldLabel(itemKey, nestedMeta)}: ${displayDetailValue(item, itemKey, nestedMeta)}`)
      .join('；');
  }

  const typed = typedDisplayValue(value, key, meta);
  if (typed) {
    return typed;
  }

  const mapped = mappedValue(value, key, meta);
  if (mapped) {
    return mapped;
  }

  if (typeof value === 'boolean') return value ? '是' : '否';
  return adminErrorFieldValue(key, value) ?? formatAdminDisplayValue(key, value) ?? String(value);
}

function DetailValue({ value, field, meta }: { value: unknown; field: string; meta?: DetailDrawerFieldMeta }) {
  if (value && typeof value === 'object' && !isAdminBetContentField(field, fieldLabel(field, meta))) {
    if (Array.isArray(value)) {
      return value.length ? <ol className="admin-detail-list">
        {value.map((item, index) => <li key={index}><DetailValue value={item} field={field} meta={meta} /></li>)}
      </ol> : <span>-</span>;
    }
    const nestedMeta = field === 'nodes' ? { ...meta, labels: { ...meta?.labels, target_type: '目标类型' } } : meta;
    return <DetailFields record={value as ApiRecord} meta={nestedMeta} nested />;
  }
  return <span className="admin-detail-value" title={value === null || value === undefined ? undefined : String(value)}>
    {displayDetailValue(value, field, meta)}
  </span>;
}

function DetailFields({ record, meta, nested = false }: { record: ApiRecord; meta?: DetailDrawerFieldMeta; nested?: boolean }) {
  return <dl className={`admin-detail-fields${nested ? ' admin-detail-fields--nested' : ''}`}>
    {Object.entries(record).map(([field, value]) => {
      const wide = (value !== null && typeof value === 'object') || String(value ?? '').length > 72;
      return <div className={`admin-detail-field${wide ? ' admin-detail-field--wide' : ''}`} key={field}>
        <dt title={field}>{fieldLabel(field, meta)}</dt>
        <dd><DetailValue value={value} field={field} meta={meta} /></dd>
      </div>;
    })}
  </dl>;
}

function recordColumns(records: ApiRecord[], meta?: DetailDrawerFieldMeta): Array<ColumnProps<ApiRecord>> {
  const keys = [...new Set(records.flatMap((record) => Object.keys(record)))];
  return keys.map((key) => ({
    dataIndex: key,
    title: fieldLabel(key, meta),
    width: 180,
    render: (value: unknown) => <DetailValue value={value} field={key} meta={meta} />
  }));
}

type DetailFieldTableProps = {
  fieldMeta?: DetailDrawerFieldMeta;
  record: ApiRecord | null;
};

export function DetailFieldTable({ fieldMeta, record }: DetailFieldTableProps) {
  return (
    <section className="admin-detail-record" aria-label="详情字段">
      {record && Object.keys(record).length
        ? <DetailFields record={record} meta={fieldMeta} />
        : <Empty description="暂无详情" />}
    </section>
  );
}

export function DetailDrawer({ detail, onClose }: DetailDrawerProps) {
  const data = detail?.data;
  const meta = detail?.fieldMeta;
  const records = Array.isArray(data) ? data : [];

  return (
    <SideSheet
      bodyStyle={{ overflowY: 'auto' }}
      className="admin-detail-drawer"
      closeOnEsc
      maskClosable={false}
      onCancel={onClose}
      title={detail?.title ?? '详情'}
      visible={detail !== null}
      width={Array.isArray(data) ? 920 : 780}
    >
      {Array.isArray(data) ? (
        <ResizableTable
          bordered
          columns={recordColumns(records, meta)}
          dataSource={records}
          pagination={false}
          rowKey={(record) => String(record?.id ?? displayDetailValue(record))}
          style={containedTableStyle}
        />
      ) : (
        <DetailFieldTable fieldMeta={meta} record={data ?? null} />
      )}
    </SideSheet>
  );
}

export type { DetailDrawerData, DetailDrawerFieldMeta, DetailFieldType };
