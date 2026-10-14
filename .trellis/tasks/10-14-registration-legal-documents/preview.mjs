import http from 'node:http'

const longCopy = 'INTERFACE TEST CONTENT ONLY. This text is not a policy or legal advice.\n\n'
  + 'Review the layout, country and language selection, version identifiers, tabs, scrolling, and acknowledgement.\n\n'.repeat(8)
const chineseCopy = '仅供界面测试，以下文字不是用户协议或隐私政策，不具有法律效力。\n\n'
  + '请检查正文排版、国家和语言切换、版本标识、标签页、滚动区域和阅读确认。\n\n'.repeat(8)

const server = http.createServer((request, response) => {
  const url = new URL(request.url || '/', 'http://127.0.0.1:16401')
  response.setHeader('Access-Control-Allow-Origin', '*')
  response.setHeader('Content-Type', 'application/json; charset=utf-8')
  if (url.pathname === '/api/v1/countries') {
    response.end(JSON.stringify({ countries: [
      { country_code: 'CN', country_name: 'China', default_locale: 'zh', supported_locales: ['zh', 'en'] },
      { country_code: 'US', country_name: 'United States', default_locale: 'en', supported_locales: ['en'] },
    ] }))
    return
  }
  if (url.pathname === '/api/v1/auth/register/config') {
    response.end(JSON.stringify({ email_code_required: true, invite_code_required: false }))
    return
  }
  if (url.pathname === '/api/v1/auth/register/legal') {
    const locale = url.searchParams.get('locale') || 'en'
    const zh = locale.startsWith('zh')
    const body = zh ? chineseCopy : longCopy
    const documentLocale = zh ? 'zh-CN' : 'en'
    response.end(JSON.stringify({
      publication_id: 12,
      published_at: '2026-10-14T08:00:00Z',
      scope_code: 'GLOBAL', locale: documentLocale, edition: 1,
      terms: { id: 21, title: zh ? '界面测试用户协议' : 'Interface test terms', body, effective_on: '2026-10-14' },
      privacy: { id: 22, title: zh ? '界面测试隐私政策' : 'Interface test privacy', body, effective_on: '2026-10-14' },
    }))
    return
  }
  response.statusCode = 404
  response.end(JSON.stringify({ code: 'NOT_FOUND', message: 'Read-only preview fixture' }))
})

server.listen(16401, '127.0.0.1')
