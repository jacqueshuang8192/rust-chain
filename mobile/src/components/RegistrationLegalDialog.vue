<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { X } from 'lucide-vue-next'
import { useModalDialog } from '@/core/modalDialog'
import { normalizeNewsRichText } from '@/core/newsRichText'
import type { RegistrationLegalBundle } from '@/api/auth'
import NewsRichText from '@/components/NewsRichText.vue'

const props = defineProps<{ bundle: RegistrationLegalBundle }>()
const emit = defineEmits<{ close: []; confirm: [] }>()
const { t } = useI18n()
const open = ref(false)
const dialog = ref<HTMLElement | null>(null)
const copy = ref<HTMLElement | null>(null)
const { trapFocus } = useModalDialog(open, dialog, '[data-legal-close]')
const selected = ref<'terms' | 'privacy'>('terms')
const visited = ref(new Set<'terms' | 'privacy'>(['terms']))
const document = computed(() => props.bundle[selected.value])
const bodyBlocks = computed(() => normalizeNewsRichText(document.value.body))
const language = computed(() => {
  if (props.bundle.locale === 'zh-CN') return t('language.zhCN')
  if (props.bundle.locale === 'en' || props.bundle.locale === 'en-US') return t('language.en')
  return props.bundle.locale
})

onMounted(() => { open.value = true })

async function select(kind: 'terms' | 'privacy') {
  if (selected.value === kind) return
  selected.value = kind
  visited.value = new Set([...visited.value, kind])
  await nextTick()
  if (copy.value) copy.value.scrollTop = 0
}

async function navigateTabs(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const kind = event.key === 'Home' ? 'terms'
    : event.key === 'End' ? 'privacy'
      : selected.value === 'terms' ? 'privacy' : 'terms'
  await select(kind)
  dialog.value?.querySelector<HTMLElement>(`#registration-legal-tab-${kind}`)?.focus()
}

async function dismiss(confirmed = false) {
  if (!open.value || (confirmed && visited.value.size < 2)) return
  open.value = false
  await nextTick()
  if (confirmed) emit('confirm')
  else emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div class="auth-pencil-page legal-overlay" @click.self="dismiss()">
      <section ref="dialog" class="legal-sheet" role="dialog" aria-modal="true"
        aria-labelledby="registration-legal-title" @keydown="trapFocus($event, () => dismiss())">
        <div class="legal-handle" aria-hidden="true" />
        <header class="legal-heading">
          <div>
            <h2 id="registration-legal-title">{{ t('auth.legalReadTitle') }}</h2>
            <small :lang="bundle.locale">{{ language }}</small>
          </div>
          <button data-legal-close type="button" :aria-label="t('common.close')" @click="dismiss()"><X :size="20" aria-hidden="true" /></button>
        </header>
        <div class="legal-tabs" role="tablist" :aria-label="t('auth.legalReadTitle')" @keydown="navigateTabs">
          <button id="registration-legal-tab-terms" type="button" role="tab" :aria-selected="selected === 'terms'"
            aria-controls="registration-legal-panel" :tabindex="selected === 'terms' ? 0 : -1"
            @click="select('terms')">{{ t('auth.legalTerms') }}</button>
          <button id="registration-legal-tab-privacy" type="button" role="tab" :aria-selected="selected === 'privacy'"
            aria-controls="registration-legal-panel" :tabindex="selected === 'privacy' ? 0 : -1"
            @click="select('privacy')">{{ t('auth.legalPrivacy') }}</button>
        </div>
        <div id="registration-legal-panel" ref="copy" class="legal-copy" tabindex="0" role="tabpanel"
          :aria-labelledby="`registration-legal-tab-${selected}`" :lang="bundle.locale">
          <h3>{{ document.title }}</h3>
          <p class="legal-date">{{ t('auth.legalEffectiveOn', { date: document.effective_on }) }}</p>
          <p class="legal-date">{{ t('auth.legalUpdatedOn', { date: bundle.published_at.slice(0, 10) }) }}</p>
          <div class="legal-body"><NewsRichText :blocks="bodyBlocks" empty-text="" /></div>
        </div>
        <footer class="legal-actions">
          <button type="button" @click="dismiss()">{{ t('common.cancel') }}</button>
          <button type="button" :disabled="visited.size < 2" @click="dismiss(true)">{{ t('auth.legalAcknowledge') }}</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.legal-overlay,
.legal-overlay * { box-sizing: border-box; }

.legal-overlay {
  position: fixed;
  inset: 0;
  z-index: calc(var(--layer-overlay, 80) + 1);
  display: grid;
  align-items: end;
  justify-items: center;
  padding: 0;
  background: var(--overlay);
  letter-spacing: 0;
}

.legal-sheet {
  --legal-inset: 20px;
  width: min(100%, 448px);
  height: min(800px, 90dvh);
  max-height: calc(100dvh - max(12px, env(safe-area-inset-top, 0px)));
  min-height: 0;
  display: grid;
  grid-template-rows: 18px auto auto minmax(0, 1fr) auto;
  overflow: hidden;
  padding-inline: env(safe-area-inset-left, 0px) env(safe-area-inset-right, 0px);
  background: var(--surface-elevated);
  color: var(--text);
  border: 1px solid var(--line);
  border-bottom: 0;
  border-radius: 24px 24px 0 0;
  box-shadow: 0 -8px 28px rgb(0 0 0 / 20%);
  animation: legal-sheet-enter 180ms ease-out;
}

.legal-handle {
  width: 38px;
  height: 4px;
  margin-top: 8px;
  justify-self: center;
  border-radius: 2px;
  background: var(--line-strong);
}

.legal-sheet button { min-width: 0; min-height: 44px; color: inherit; overflow-wrap: anywhere; }
.legal-heading { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--legal-inset) 14px; gap: 12px; }
.legal-heading > div { min-width: 0; overflow-wrap: anywhere; }
.legal-heading h2 { margin: 0 0 2px; font-size: 19px; line-height: 26px; }
.legal-heading small { color: var(--muted); font-size: 12px; }
.legal-heading button { display: grid; place-items: center; flex: 0 0 44px; height: 44px; padding: 0; border: 1px solid var(--line); border-radius: 50%; background: var(--surface-2); }
.legal-tabs { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); margin: 0 var(--legal-inset); padding: 4px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-2); }
.legal-tabs button { padding: 8px 6px; border: 0; border-radius: 5px; background: transparent; color: var(--muted-strong); font-size: 13px; line-height: 18px; }
.legal-tabs button[aria-selected="true"] { background: var(--surface-elevated); color: var(--positive); }
.legal-copy { min-height: 0; overflow-y: auto; overscroll-behavior: contain; scrollbar-width: thin; padding: 22px var(--legal-inset) 28px; overflow-wrap: anywhere; }
.legal-copy h3 { font-size: 17px; line-height: 25px; margin: 0 0 10px; }
.legal-date { color: var(--muted); font-size: 11px; line-height: 18px; margin: 0 0 2px; }
.legal-body { margin-top: 22px; }
.legal-body :deep(.news-rich-text) { margin-top: 0; font-size: 14px; }
.legal-body :deep(p) { white-space: pre-wrap; }
.legal-actions { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 2fr); gap: 10px; padding: 14px var(--legal-inset) max(16px, env(safe-area-inset-bottom, 0px)); border-top: 1px solid var(--line); }
.legal-actions button { min-height: 50px; padding: 10px 12px; border-radius: 8px; border: 1px solid var(--line); background: var(--surface-2); font-size: 14px; font-weight: 650; line-height: 20px; }
.legal-actions button:last-child { border-color: var(--accent); background: var(--accent); color: var(--on-accent); }
.legal-actions button:disabled { border-color: var(--line); background: var(--surface-2); color: var(--muted); cursor: not-allowed; }
.legal-sheet :focus-visible { outline: 2px solid var(--positive); outline-offset: 2px; }
.legal-copy:focus-visible { outline-offset: -4px; }

@keyframes legal-sheet-enter {
  from { transform: translateY(24px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

@media (max-width: 340px) { .legal-sheet { --legal-inset: 16px; } }
@media (prefers-reduced-motion: reduce) { .legal-sheet { animation: none; } }
</style>
