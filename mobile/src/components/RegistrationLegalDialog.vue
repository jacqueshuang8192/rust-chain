<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { X } from 'lucide-vue-next'
import { useModalDialog } from '@/core/modalDialog'
import { normalizeNewsRichText } from '@/core/newsRichText'
import type { RegistrationLegalBundle } from '@/api/auth'
import NewsRichText from '@/components/NewsRichText.vue'

const props = defineProps<{ bundle: RegistrationLegalBundle }>()
const emit = defineEmits<{ close: []; confirm: [] }>()
const { t } = useI18n()
const open = ref(true)
const dialog = ref<HTMLElement | null>(null)
const { trapFocus } = useModalDialog(open, dialog, '[data-legal-initial]')
const selected = ref<'terms' | 'privacy'>('terms')
const visited = ref(new Set<'terms' | 'privacy'>(['terms']))
const document = computed(() => props.bundle[selected.value])
const bodyBlocks = computed(() => normalizeNewsRichText(document.value.body))

function select(kind: 'terms' | 'privacy') {
  selected.value = kind
  visited.value = new Set([...visited.value, kind])
}
</script>

<template>
  <Teleport to="body">
    <div class="registration-legal-backdrop" @click.self="emit('close')">
      <section ref="dialog" class="registration-legal-dialog" role="dialog" aria-modal="true"
        aria-labelledby="registration-legal-title" @keydown="trapFocus($event, () => emit('close'))">
        <header class="registration-legal-heading">
          <div>
            <h2 id="registration-legal-title">{{ t('auth.legalReadTitle') }}</h2>
            <small>{{ bundle.locale }}</small>
          </div>
          <button type="button" :aria-label="t('common.close')" @click="emit('close')"><X :size="20" /></button>
        </header>
        <div class="registration-legal-tabs" role="tablist" :aria-label="t('auth.legalReadTitle')">
          <button data-legal-initial type="button" role="tab" :aria-selected="selected === 'terms'"
            @click="select('terms')">{{ t('auth.legalTerms') }}</button>
          <button type="button" role="tab" :aria-selected="selected === 'privacy'"
            @click="select('privacy')">{{ t('auth.legalPrivacy') }}</button>
        </div>
        <div class="registration-legal-copy" tabindex="0" role="tabpanel">
          <h3>{{ document.title }}</h3>
          <p class="registration-legal-date">{{ t('auth.legalEffectiveOn', { date: document.effective_on }) }}</p>
          <p class="registration-legal-date">{{ t('auth.legalUpdatedOn', { date: bundle.published_at.slice(0, 10) }) }}</p>
          <div class="registration-legal-body"><NewsRichText :blocks="bodyBlocks" empty-text="" /></div>
        </div>
        <footer class="registration-legal-actions">
          <button type="button" @click="emit('close')">{{ t('common.cancel') }}</button>
          <button type="button" :disabled="visited.size < 2" @click="emit('confirm')">{{ t('auth.legalAcknowledge') }}</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.registration-legal-backdrop { position: fixed; inset: 0; z-index: calc(var(--layer-overlay) + 1); background: var(--overlay); display: grid; place-items: center; padding: 16px; }
.registration-legal-dialog { width: min(100%, 560px); max-height: min(760px, calc(100dvh - 32px)); min-height: 0; display: grid; grid-template-rows: auto auto minmax(0, 1fr) auto; overflow: hidden; background: var(--surface-elevated); border: 1px solid var(--line); border-radius: 8px; box-shadow: var(--shadow-soft); }
.registration-legal-heading { display: flex; align-items: center; justify-content: space-between; padding: 20px 20px 12px; gap: 12px; }
.registration-legal-heading h2 { margin: 0; font-size: 17px; line-height: 24px; }
.registration-legal-heading small { color: var(--muted); font-size: 12px; }
.registration-legal-heading button { display: grid; place-items: center; width: 44px; height: 44px; border: 0; background: transparent; color: inherit; }
.registration-legal-tabs { display: grid; grid-template-columns: repeat(2, 1fr); margin: 0 20px; background: var(--surface-2); border-radius: 6px; padding: 4px; }
.registration-legal-tabs button { min-height: 44px; background: transparent; border: 0; border-radius: 4px; color: var(--muted); }
.registration-legal-tabs button[aria-selected="true"] { background: var(--surface-elevated); color: var(--text); box-shadow: var(--shadow-soft); }
.registration-legal-copy { overflow-y: auto; overscroll-behavior: contain; padding: 18px 20px 24px; outline-offset: -4px; }
.registration-legal-copy h3 { font-size: 15px; margin: 0 0 6px; }
.registration-legal-date { font-size: 12px; color: var(--muted); margin: 0 0 4px; }
.registration-legal-body { margin-top: 18px; overflow-wrap: anywhere; }
.registration-legal-body :deep(.news-rich-text) { margin-top: 0; font-size: 14px; line-height: 1.65; }
.registration-legal-body :deep(p) { white-space: pre-wrap; }
.registration-legal-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 20px calc(12px + env(safe-area-inset-bottom)); border-top: 1px solid var(--line); }
.registration-legal-actions button { min-height: 44px; padding: 0 16px; border-radius: 5px; border: 1px solid var(--line-strong); background: var(--surface-elevated); color: inherit; }
.registration-legal-actions button:last-child { border-color: var(--accent); background: var(--accent); color: var(--on-accent); }
.registration-legal-actions button:disabled { opacity: .5; }
@media (max-width: 440px) { .registration-legal-backdrop { padding: 0; align-items: end; } .registration-legal-dialog { max-height: 94dvh; width: 100%; border-radius: 8px 8px 0 0; } }
</style>
