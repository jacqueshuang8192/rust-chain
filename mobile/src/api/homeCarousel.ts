import { client, publicApiRequestConfig, requestUrl } from './client'
import { currentApiLocale } from '../i18n'
import { parseHomeCarouselResponse, type HomeCarouselSlide } from '../core/homeCarouselContract'

export type {
  HomeCarouselSlide,
  HomeCarouselTarget,
  HomeCarouselTargetType,
} from '../core/homeCarouselContract'
export { HomeCarouselContractError } from '../core/homeCarouselContract'

export async function fetchHomeCarousel(): Promise<HomeCarouselSlide[]> {
  const response = await client.get<unknown>(requestUrl('/home/carousel'), publicApiRequestConfig({
    params: { locale: currentApiLocale() },
  }))
  return parseHomeCarouselResponse(response.data)
}
