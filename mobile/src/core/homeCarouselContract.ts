import { requiredSafeInteger } from './numeric.ts'

export type HomeCarouselTargetType = 'news_detail' | 'news_center' | 'referrals'

export interface HomeCarouselTarget {
  type: HomeCarouselTargetType
  id?: number
}

export interface HomeCarouselSlide {
  id: string
  position: number
  enabled: boolean
  locale?: string
  title: string
  description: string
  imageUrl: string
  target: HomeCarouselTarget
}

export class HomeCarouselContractError extends TypeError {
  constructor(field: string) {
    super(`Invalid home carousel ${field}`)
    this.name = 'HomeCarouselContractError'
  }
}

export function parseHomeCarouselResponse(value: unknown): HomeCarouselSlide[] {
  const envelope = record(value, 'response')
  assertKeys(envelope, ['slides'], 'response')
  if (!Array.isArray(envelope.slides) || envelope.slides.length > 12) {
    throw new HomeCarouselContractError('slides')
  }

  const slides = envelope.slides.map((item, index) => parseSlide(item, index))
  if (new Set(slides.map((slide) => slide.id)).size !== slides.length) {
    throw new HomeCarouselContractError('slide ids')
  }
  if (slides.some((slide, index) => index > 0 && slide.position <= slides[index - 1].position)) {
    throw new HomeCarouselContractError('slide positions')
  }
  return slides
}

function parseSlide(value: unknown, index: number): HomeCarouselSlide {
  const item = record(value, `slides[${index}]`)
  assertKeys(item, ['id', 'position', 'enabled', 'locale', 'title', 'description', 'image_url', 'target'], `slides[${index}]`)
  const id = text(item.id, `slides[${index}].id`)
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(id)) throw new HomeCarouselContractError(`slides[${index}].id`)
  const position = requiredSafeInteger(item.position, `slides[${index}].position`, 0, 11)
  if (typeof item.enabled !== 'boolean') throw new HomeCarouselContractError(`slides[${index}].enabled`)
  const locale = item.locale == null ? undefined : text(item.locale, `slides[${index}].locale`)
  const title = text(item.title, `slides[${index}].title`)
  const description = text(item.description, `slides[${index}].description`)
  const imageUrl = text(item.image_url, `slides[${index}].image_url`)
  if ([...title.trim()].length > 80 || [...description.trim()].length > 180 || imageUrl.length > 2048
    || (locale && !/^[a-z]{2}(?:-[A-Z]{2})?$/.test(locale))) {
    throw new HomeCarouselContractError(`slides[${index}]`)
  }
  try {
    const parsed = new URL(imageUrl)
    if (!['http:', 'https:'].includes(parsed.protocol) || !parsed.hostname || parsed.username || parsed.password) {
      throw new Error('unsafe')
    }
  } catch {
    throw new HomeCarouselContractError(`slides[${index}].image_url`)
  }
  const target = parseTarget(item.target, index)
  return { id, position, enabled: item.enabled, locale, title, description, imageUrl, target }
}

function parseTarget(value: unknown, index: number): HomeCarouselTarget {
  const target = record(value, `slides[${index}].target`)
  assertKeys(target, ['type', 'id'], `slides[${index}].target`)
  if (target.type !== 'news_detail' && target.type !== 'news_center' && target.type !== 'referrals') {
    throw new HomeCarouselContractError(`slides[${index}].target.type`)
  }
  if (target.type === 'news_detail') {
    return { type: target.type, id: requiredSafeInteger(target.id, `slides[${index}].target.id`, 1) }
  }
  if (target.id != null) throw new HomeCarouselContractError(`slides[${index}].target.id`)
  return { type: target.type }
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new HomeCarouselContractError(field)
  }
  return value as Record<string, unknown>
}

function assertKeys(value: Record<string, unknown>, keys: string[], field: string): void {
  const allowed = new Set(keys)
  if (Object.keys(value).some((key) => !allowed.has(key))) {
    throw new HomeCarouselContractError(field)
  }
}

function text(value: unknown, field: string): string {
  if (typeof value !== 'string' || !value.trim()) throw new HomeCarouselContractError(field)
  return value
}
