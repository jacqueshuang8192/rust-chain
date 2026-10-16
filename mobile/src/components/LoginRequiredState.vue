<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { LockKeyhole } from 'lucide-vue-next'

const props = defineProps<{ description?: string }>()

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const resolvedDescription = computed(() => props.description || t('common.loginRequiredDescription'))

function goToLogin() {
  void router.push({ name: 'login', query: { redirect: route.fullPath } })
}
</script>

<template>
  <section
    class="login-required"
    role="group"
    :aria-label="t('common.loginRequiredTitle')"
  >
    <span class="login-required__icon" aria-hidden="true"><LockKeyhole :size="22" /></span>
    <div class="login-required__copy">
      <strong>{{ t('common.loginRequiredTitle') }}</strong>
      <p>{{ resolvedDescription }}</p>
    </div>
    <button class="button button--primary" type="button" @click="goToLogin">
      {{ t('common.loginNow') }}
    </button>
  </section>
</template>

<style scoped>
.login-required {
  display: grid;
  align-items: center;
  gap: 16px;
  color: var(--muted-strong);
}

.login-required__icon {
  display: grid;
  place-items: center;
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: var(--surface-2);
  color: var(--positive);
}

.login-required__copy {
  display: grid;
  gap: 10px;
  min-width: 0;
  overflow-wrap: anywhere;
}

.login-required strong {
  color: var(--ink);
  font-size: 22px;
  line-height: 30px;
}

.login-required p {
  font-size: 15px;
  line-height: 24px;
  margin: 0;
}

.login-required .button {
  min-height: 46px;
  padding-inline: 28px;
  white-space: nowrap;
}

.login-required:not(.wallet-login-prompt) {
  align-content: center;
  justify-items: center;
  min-height: calc(100dvh - var(--login-required-offset, 160px) - env(safe-area-inset-bottom));
  padding: 32px 16px;
  background: transparent;
  border: 0;
  text-align: center;
}

.login-required:not(.wallet-login-prompt) .login-required__copy { max-width: 280px; text-wrap: balance; }

.login-required:not(.wallet-login-prompt) .button {
  width: min(100%, 240px);
  margin-top: 12px;
  border: 0;
  box-shadow:none;
  border-radius: 999px;
  background: var(--accent);
  color: var(--on-accent);
}
</style>
