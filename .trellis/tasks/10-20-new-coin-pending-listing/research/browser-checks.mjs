const task = await taskSpace(26); const page = task.page('p1');
await page.evaluate(() => { window.__phaseQA.project.lifecycle_status = 'distribution'; });
await page.click("loc=role:button[name='刷新项目阶段']", { label: 'Restore distribution fixture' });
await page.waitForFunction(() => document.querySelector('.new-coin-detail-visual header > b')?.textContent === '等待派发');
const results = [];
for (const locale of ['zh-CN', 'en']) {
  for (const theme of ['light', 'dark']) {
    for (const width of [320, 390, 448]) {
      await page.cdp('Emulation.setDeviceMetricsOverride', { width, height: 844, deviceScaleFactor: 1, mobile: true });
      await page.evaluate(async ({ locale, theme }) => {
        const url = performance.getEntriesByType('resource').map(r => r.name).find(u => /\/src\/i18n\/index\.ts/.test(u));
        const { i18n } = await import(url); i18n.global.locale.value = locale;
        document.documentElement.dataset.theme = theme;
      }, { locale, theme });
      await page.waitForFunction(({ locale }) => document.querySelector('.new-coin-detail-stage-note button')?.textContent.includes(locale === 'en' ? 'Refresh' : '刷新'), { locale });
      results.push(await page.evaluate(({ locale, theme, width }) => ({
        locale, theme, width,
        overflow: document.documentElement.scrollWidth > innerWidth,
        stages: [...document.querySelectorAll('.new-coin-detail-stages li')].map(e => e.textContent.trim()),
        active: document.querySelector('.new-coin-detail-stages li.active')?.textContent.trim(),
        plannedLabels: [...document.querySelectorAll('.token-rules dt')].filter(e => /计划|Planned/.test(e.textContent)).length,
        hiddenCopyVisible: /配置时区|自动推进|Timezone|Automatic transition/.test(document.querySelector('.token-rules')?.textContent || ''),
        targets: [...document.querySelectorAll('.new-coin-detail-stage-note button')].map(e => ({ height: e.getBoundingClientRect().height, width: e.getBoundingClientRect().width })),
        overlay: !!document.querySelector('vite-error-overlay'),
      }), { locale, theme, width }));
      if (width === 320) console.log(await page.screenshot({ path: `/tmp/new-coin-${locale}-${theme}-320.png`, fullPage: true }));
    }
  }
}
console.log(JSON.stringify(results));
const fs = await import('node:fs/promises');
await fs.writeFile('/Users/huangkunhuang/Public/程序工程目录/复合工程/rust-chain/.trellis/tasks/10-20-new-coin-pending-listing/research/browser-results.json', JSON.stringify(results, null, 2));
