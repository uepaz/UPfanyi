<script setup lang="ts">
import { computed, onScopeDispose, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import AppSelect from './AppSelect.vue'
import AppCombobox from './AppCombobox.vue'
import { desktop } from '../services/desktop'
import Notice from './Notice.vue'
import { useI18n } from '../i18n'
import { aiConfigStatus, endpointForProtocol } from '../services/aiConfig'
import type { AiChannels, AiConfig } from '../types/desktop'

const props = defineProps<{
  config: AiConfig
  channels: AiChannels
  disabled?: boolean
  test?: { pending: boolean; error?: string; success?: string }
  models?: { pending: boolean; error: string; items: string[]; loaded: boolean }
}>()
const emit = defineEmits<{
  update: [value: AiConfig]
  channel: [id: string]
  add: []
  rename: []
  delete: []
  test: []
  models: []
}>()
const { t, failure } = useI18n()
const visibleSecret = ref(false)
const status = computed(() => aiConfigStatus(props.config))
const channel = computed(() => props.channels.activeId)
const channelOptions = computed(() => [
  { value: 'upfanyi', label: t('upfanyiOfficialAi') },
  { value: 'add-channel', label: t('addAiChannel'), action: 'add' },
  ...props.channels.channels
    .filter((item) => item.id !== 'upfanyi')
    .map((item) => ({ value: item.id, label: item.name })),
])

function inputValue(event: Event) {
  return (event.target as HTMLInputElement).value
}
function setField(name: Exclude<keyof AiConfig, 'protocol'>, event: Event) {
  emit('update', { ...props.config, [name]: inputValue(event) })
}
function setProtocol(protocol: string) {
  if (protocol !== 'openai' && protocol !== 'openai-responses') return
  const endpoint = endpointForProtocol(props.config, protocol)
  emit('update', { ...props.config, protocol, endpoint })
}
const officialOpening = ref(false)
const officialOpenFailed = ref(false)
let websiteGeneration = 0
watch(
  channel,
  () => {
    ++websiteGeneration
    officialOpening.value = false
    officialOpenFailed.value = false
  },
  { flush: 'sync' },
)
onScopeDispose(() => ++websiteGeneration)
async function openOfficialWebsite() {
  if (officialOpening.value || props.disabled || channel.value !== 'upfanyi') return
  const current = ++websiteGeneration
  officialOpening.value = true
  officialOpenFailed.value = false
  try {
    await desktop.openOfficialWebsite()
  } catch {
    // 仅反馈当前渠道的打开结果，不将系统错误或过时请求覆盖到新草稿。
    if (current === websiteGeneration) officialOpenFailed.value = true
  } finally {
    if (current === websiteGeneration) officialOpening.value = false
  }
}
</script>

<template>
  <div class="ai-config-form">
    <h2>{{ t('ai') }}</h2>
    <p class="ai-state" :class="{ incomplete: !status.configured }">
      {{ status.configured ? t('configured') : t('notConfigured') }}
      <template v-if="status.missing.length">
        · {{ t('missingFields') }} {{ status.missing.map((name) => t(name)).join('、') }}
      </template>
    </p>
    <div class="ai-fields">
      <div class="ai-channel-row">
        <label class="field" for="ai-channel">
          <span>{{ t('aiChannel') }}</span>
          <AppSelect
            id="ai-channel"
            :model-value="channel"
            :label="t('aiChannel')"
            described-by="ai-channel-hint"
            :disabled="disabled"
            :options="channelOptions"
            @update:model-value="emit('channel', $event)"
            @action="emit('add')"
          />
          <small id="ai-channel-hint">{{ t('aiChannelHint') }}</small>
          <div v-if="channel !== 'upfanyi'" class="channel-actions">
            <button
              class="button subtle small"
              type="button"
              :disabled="disabled"
              @click="emit('rename')"
            >
              {{ t('renameAiChannel') }}
            </button>
            <button
              class="button subtle small channel-delete"
              type="button"
              :disabled="disabled"
              @click="emit('delete')"
            >
              {{ t('deleteAiChannel') }}
            </button>
          </div>
        </label>
        <div v-if="channel === 'upfanyi'" class="official-website">
          <button
            class="official-website-button"
            type="button"
            :disabled="disabled || officialOpening"
            @click="openOfficialWebsite"
          >
            <span v-if="officialOpening" class="spinner"></span>
            {{ officialOpening ? t('openingOfficialWebsite') : t('visitOfficialWebsite') }}
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path
                d="M14 3h7v7m0-7L10 14M10 3H4a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-6"
              />
            </svg>
          </button>
          <Notice v-if="officialOpenFailed" kind="error">{{
            t('officialWebsiteOpenError')
          }}</Notice>
        </div>
        <label v-else class="field" for="ai-endpoint">
          <span
            >{{ t('endpoint') }} <small>{{ t('required') }}</small></span
          >
          <input
            id="ai-endpoint"
            :value="config.endpoint"
            autocomplete="off"
            spellcheck="false"
            :placeholder="t('endpointPlaceholder')"
            aria-describedby="ai-endpoint-hint"
            @input="setField('endpoint', $event)"
          />
          <small id="ai-endpoint-hint">{{ t('endpointHint') }}</small>
        </label>
      </div>
      <div class="form-grid">
        <label class="field" for="ai-protocol">
          <span
            >{{ t('protocol') }} <small>{{ t('required') }}</small></span
          >
          <AppSelect
            id="ai-protocol"
            model-value="openai"
            :label="t('protocol')"
            :disabled="disabled"
            :options="[{ value: 'openai', label: t('openaiCompatible') }]"
          />
        </label>
        <label class="field" for="ai-interface">
          <span
            >{{ t('interfaceType') }} <small>{{ t('required') }}</small></span
          >
          <AppSelect
            id="ai-interface"
            :model-value="config.protocol"
            :label="t('interfaceType')"
            :disabled="disabled"
            :options="[
              { value: 'openai', label: 'Chat Completions' },
              { value: 'openai-responses', label: 'Responses' },
            ]"
            @update:model-value="setProtocol"
          />
        </label>
      </div>
      <div class="field">
        <label for="ai-api-key"
          >{{ t('apiKey') }} <small>{{ t('optional') }}</small></label
        >
        <div class="secret-input">
          <input
            id="ai-api-key"
            :value="config.apiKey"
            :type="visibleSecret ? 'text' : 'password'"
            autocomplete="off"
            spellcheck="false"
            @input="setField('apiKey', $event)"
          />
          <button
            class="icon-button"
            type="button"
            :aria-label="visibleSecret ? t('hideSecret') : t('showSecret')"
            :aria-pressed="visibleSecret"
            @click="visibleSecret = !visibleSecret"
          >
            <AppIcon name="eye" :size="17" />
          </button>
        </div>
        <small>{{ t('apiKeyHint') }}</small>
      </div>
      <div class="field">
        <label for="ai-model"
          >{{ t('model') }} <small>{{ t('required') }}</small></label
        >
        <div class="model-input">
          <AppCombobox
            id="ai-model"
            :model-value="config.model"
            :options="models?.items ?? []"
            :label="t('model')"
            :placeholder="t('modelPlaceholder')"
            described-by="ai-model-hint ai-model-status"
            :disabled="disabled"
            @update:model-value="emit('update', { ...config, model: $event })"
          />
          <button
            class="button subtle"
            type="button"
            :disabled="disabled || models?.pending"
            @click="emit('models')"
          >
            <span v-if="models?.pending" class="spinner"></span>
            {{
              models?.pending
                ? t('loadingModels')
                : models?.error
                  ? t('retryModels')
                  : t('loadModels')
            }}
          </button>
        </div>
        <small id="ai-model-hint">{{ t('modelHint') }}</small>
        <div id="ai-model-status" aria-live="polite" :aria-busy="models?.pending ?? false">
          <Notice v-if="models?.error" kind="error">
            {{ t('modelsError') }} · {{ failure(models.error) }} {{ t('modelManualHint') }}
          </Notice>
          <small v-else-if="models?.loaded">
            {{
              models.items.length ? `${t('modelsLoaded')} ${models.items.length}` : t('modelsEmpty')
            }}
          </small>
        </div>
      </div>
      <label class="field" for="ai-system-prompt">
        <span
          >{{ t('systemPrompt') }} <small>{{ t('optional') }}</small></span
        >
        <textarea
          id="ai-system-prompt"
          :value="config.systemPrompt"
          rows="3"
          @input="setField('systemPrompt', $event)"
        ></textarea>
      </label>
    </div>
    <p class="helper muted">{{ t('aiHint') }}</p>
    <Notice v-if="test?.error" kind="error">{{ failure(test.error) }}</Notice>
    <Notice v-if="test?.success" kind="success">{{ t('testSuccess') }} · {{ test.success }}</Notice>
    <div class="test-connection">
      <button
        class="button subtle"
        type="button"
        :disabled="disabled || test?.pending"
        @click="emit('test')"
      >
        <span v-if="test?.pending" class="spinner"></span
        >{{ test?.pending ? t('testing') : t('connectionTest') }}
      </button>
      <p class="helper muted">{{ t('testHint') }}</p>
    </div>
  </div>
</template>
