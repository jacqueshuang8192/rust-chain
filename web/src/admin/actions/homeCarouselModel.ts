import { ContractError, apiRequest } from '../../api/client';
import type { SettingsFieldDefinition, SettingsValidationIssue } from '../settings';

export const homeCarouselApiPath = '/admin/api/v1/home/carousel';
export const maxSlides = 12;
export type HomeCarouselTargetType = 'news_detail' | 'news_center' | 'referrals';
export type CarouselSlideDraft = {
  id: string;
  enabled: boolean;
  locale: string;
  title: string;
  description: string;
  imageUrl: string;
  targetType: HomeCarouselTargetType;
  newsId: string;
};
export type HomeCarouselForm = { revision: number; slides: CarouselSlideDraft[] };
export const targetOptions = [
  { value: 'referrals', label: '邀请好友' },
  { value: 'news_center', label: '新闻中心' },
  { value: 'news_detail', label: '新闻详情' }
];

export function emptySlide(): CarouselSlideDraft {
  return { id: crypto.randomUUID(), enabled: false, locale: '', title: '', description: '', imageUrl: '', targetType: 'referrals', newsId: '' };
}

export function safeCarouselImage(value: string): boolean {
  try {
    const parsed = new URL(value);
    return value.length <= 2048 && ['https:', 'http:'].includes(parsed.protocol)
      && Boolean(parsed.hostname) && !parsed.username && !parsed.password;
  } catch {
    return false;
  }
}

export function validateCarouselForm(form: HomeCarouselForm): SettingsValidationIssue[] {
  const issues: SettingsValidationIssue[] = [];
  if (!Number.isSafeInteger(form.revision) || form.revision < 0 || form.slides.length > maxSlides) {
    issues.push({ key: 'config', field: '配置', message: '版本无效或轮播项超过 12 项。' });
  }
  const ids = new Set<string>();
  form.slides.forEach((slide, index) => {
    const add = (key: string, field: string, message: string) =>
      issues.push({ key: `${slide.id}-${key}`, field: `第 ${index + 1} 项${field}`, message });
    if (!/^[A-Za-z0-9_-]{1,64}$/.test(slide.id) || ids.has(slide.id)) add('id', '标识', '轮播标识无效或重复。');
    ids.add(slide.id);
    if (!slide.title.trim() || [...slide.title.trim()].length > 80) add('title', '标题', '请输入 1-80 个字符。');
    if (!slide.description.trim() || [...slide.description.trim()].length > 180) add('description', '描述', '请输入 1-180 个字符。');
    if (!safeCarouselImage(slide.imageUrl.trim())) add('image', '图片', '请上传图片或填写无账号密码的 http(s) 图片地址。');
    if (slide.locale && !/^[a-z]{2}(?:-[A-Z]{2})?$/.test(slide.locale)) add('locale', '语言', '语言格式无效。');
    if (!targetOptions.some((option) => option.value === slide.targetType)) add('target', '目标', '请选择站内目标。');
    if (slide.targetType === 'news_detail' && (!/^[1-9]\d*$/.test(slide.newsId)
      || !Number.isSafeInteger(Number(slide.newsId)))) add('news', '新闻 ID', '请输入安全范围内的正整数 ID。');
  });
  return issues;
}

function invalid(): never {
  throw new ContractError('首页轮播响应格式无效，请重新加载。', { path: homeCarouselApiPath });
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return invalid();
  return value as Record<string, unknown>;
}

function text(value: unknown): string {
  if (typeof value !== 'string') return invalid();
  return value;
}

export function parseCarouselConfig(value: unknown): HomeCarouselForm {
  const data = record(value);
  if (!Number.isSafeInteger(data.revision) || (data.revision as number) < 0
    || !Array.isArray(data.slides) || data.slides.length > maxSlides) return invalid();
  let previousPosition = -1;
  const form: HomeCarouselForm = {
    revision: data.revision as number,
    slides: data.slides.map((item: unknown) => {
      const slide = record(item);
      const target = record(slide.target);
      if (!Number.isSafeInteger(slide.position) || (slide.position as number) <= previousPosition
        || (slide.position as number) > 11 || typeof slide.enabled !== 'boolean') return invalid();
      previousPosition = slide.position as number;
      if (target.type === 'news_detail') {
        if (!Number.isSafeInteger(target.id) || (target.id as number) < 1) return invalid();
      } else if (target.id != null) return invalid();
      return {
        id: text(slide.id), enabled: slide.enabled, locale: slide.locale == null ? '' : text(slide.locale),
        title: text(slide.title), description: text(slide.description), imageUrl: text(slide.image_url),
        targetType: text(target.type) as HomeCarouselTargetType,
        newsId: target.type === 'news_detail' ? String(target.id) : ''
      };
    })
  };
  if (validateCarouselForm(form).length) return invalid();
  return form;
}

export async function loadCarouselConfig(): Promise<HomeCarouselForm> {
  return parseCarouselConfig(await apiRequest<unknown>(homeCarouselApiPath));
}

export async function saveCarouselConfig(form: HomeCarouselForm, reason: string): Promise<HomeCarouselForm> {
  if (validateCarouselForm(form).length || !reason.trim() || [...reason.trim()].length > 512) {
    throw new Error('请检查轮播字段，并填写不超过 512 字的操作原因。');
  }
  return parseCarouselConfig(await apiRequest<unknown>(homeCarouselApiPath, {
    method: 'PATCH',
    body: JSON.stringify({
      expected_revision: form.revision,
      reason: reason.trim(),
      slides: form.slides.map((slide, position) => ({
        id: slide.id, position, enabled: slide.enabled, locale: slide.locale || null,
        title: slide.title.trim(), description: slide.description.trim(), image_url: slide.imageUrl.trim(),
        target: { type: slide.targetType, id: slide.targetType === 'news_detail' ? Number(slide.newsId) : null }
      }))
    })
  }));
}

export function carouselFields(before: HomeCarouselForm, after: HomeCarouselForm): SettingsFieldDefinition<HomeCarouselForm>[] {
  const fields: SettingsFieldDefinition<HomeCarouselForm>[] = [{
    key: 'order', field: '显示顺序',
    read: (form) => form.slides.map((slide) => slide.id),
    format: (value) => (value as string[]).join(' → ') || '无'
  }];
  const ids = [...new Set([...before.slides, ...after.slides].map((slide) => slide.id))];
  const labels: Array<[keyof CarouselSlideDraft, string]> = [
    ['title', '标题'], ['description', '描述'], ['enabled', '启用'], ['locale', '语言'],
    ['imageUrl', '图片'], ['targetType', '目标类型'], ['newsId', '新闻 ID']
  ];
  ids.forEach((id, index) => labels.forEach(([key, label]) => fields.push({
    key: `${id}-${key}`, field: `轮播 ${index + 1} · ${label}`,
    read: (form) => form.slides.find((slide) => slide.id === id)?.[key] ?? null,
    format: (value) => value === null ? '无' : key === 'enabled' ? (value ? '是' : '否')
      : key === 'targetType' ? targetOptions.find((option) => option.value === value)?.label ?? String(value)
      : key === 'locale' && !value ? '全部语言' : String(value)
  })));
  return fields;
}
