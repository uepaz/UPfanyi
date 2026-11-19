<script setup lang="ts">
import { computed, nextTick, onScopeDispose, ref, watch } from 'vue'
import AppIcon from '../components/AppIcon.vue'
import AppSelect from '../components/AppSelect.vue'
import ShortcutRecorder from '../components/ShortcutRecorder.vue'
import AiConfigForm from '../components/AiConfigForm.vue'
import ChannelNameDialog from '../components/ChannelNameDialog.vue'
import ConfirmAction from '../components/ConfirmAction.vue'
import Notice from '../components/Notice.vue'
import { useI18n } from '../i18n'
import { desktop, errorMessage } from '../services/desktop'
import { useAiModels } from '../composables/useAiModels'
import { onSelectScopeChange, provideSelectScope } from '../composables/useSelectScope'
import { channelNameError, emptyAiConfig, OFFICIAL_AI_CHANNEL_ID } from '../services/aiConfig'
import type { AiChannels, AiConfig, Bootstrap, Settings } from '../types/desktop'
const props = defineProps<{ data: Bootstrap }>()
const emit = defineEmits<{ saved: [value: Bootstrap] }>()
const { t, failure } = useI18n()
type Section = 'preferences' | 'ai' | 'desktop'
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}
const section = ref<Section>('preferences')
provideSelectScope(section)
const content = ref<HTMLElement | null>(null)
const editorVersion = ref(0)
const draft = ref<Settings>(clone(props.data.settings))
const aiChannels = ref<AiChannels>(clone(props.data.aiChannels))
const activeChannel = computed(() =>
  aiChannels.value.channels.find((item) => item.id === aiChannels.value.activeId)!,
)
const aiConfig = computed(() => activeChannel.value.config)
const models = useAiModels(() => aiConfig.value)
const changed = computed(
  () =>
    JSON.stringify(draft.value) !== JSON.stringify(props.data.settings) ||
    JSON.stringify(aiChannels.value) !== JSON.stringify(props.data.aiChannels),
)
const connectionTest = ref<{ pending: boolean; error?: string; success?: string }>()
const saving = ref(false)
const error = ref('')
const message = ref('')
let testGeneration = 0
const nameDialog = ref<{ id?: string; initialName: string; error: string }>()
const deleteChannelId = ref<string>()
function closeDialogs() {
  nameDialog.value = undefined
  deleteChannelId.value = undefined
}
onSelectScopeChange(closeDialogs)
watch(section, closeDialogs)
function invalidateRequests() {
  ++testGeneration
  models.reset()
  connectionTest.value = undefined
}
function reset() {
  invalidateRequests()
  closeDialogs()
  ++editorVersion.value
  draft.value = clone(props.data.settings)
  aiChannels.value = clone(props.data.aiChannels)
  connectionTest.value = undefined
  error.value = ''
  message.value = ''
}
watch(() => props.data, reset)
onScopeDispose(() => ++testGeneration)
watch(section, () => {
  if (content.value) content.value.scrollTop = 0
})
function replaceConfig(value: AiConfig) {
  // 任意编辑都使上次测试失效，即使随后改回相同内容也不能接受旧响应。
  ++testGeneration
  activeChannel.value.config = value
  connectionTest.value = undefined
}
function selectChannel(id: string) {
  if (
    saving.value ||
    id === aiChannels.value.activeId ||
    !aiChannels.value.channels.some((item) => item.id === id)
  )
    return
  // 两个渠道可使用完全相同的服务配置，仍以身份切换作废旧请求。
  invalidateRequests()
  aiChannels.value.activeId = id
}
function openNameDialog(rename = false) {
  if (saving.value || (rename && activeChannel.value.id === OFFICIAL_AI_CHANNEL_ID)) return
  nameDialog.value = {
    id: rename ? activeChannel.value.id : undefined,
    initialName: rename ? activeChannel.value.name : '',
    error: '',
  }
}
function confirmName(name: string) {
  const dialog = nameDialog.value
  if (!dialog || saving.value) return
  const errorKey = channelNameError(name, aiChannels.value.channels, dialog.id)
  if (errorKey) {
    dialog.error = errorKey
    return
  }
  if (dialog.id) {
    const item = aiChannels.value.channels.find((item) => item.id === dialog.id)
    if (item && item.id !== OFFICIAL_AI_CHANNEL_ID) item.name = name.trim()
  } else {
    const id = crypto.randomUUID()
    aiChannels.value.channels.push({ id, name: name.trim(), config: emptyAiConfig() })
    selectChannel(id)
  }
  nameDialog.value = undefined
}
function requestDelete() {
  if (!saving.value && activeChannel.value.id !== OFFICIAL_AI_CHANNEL_ID)
    deleteChannelId.value = activeChannel.value.id
}
function confirmDelete() {
  const id = deleteChannelId.value
  if (!id || saving.value || id === OFFICIAL_AI_CHANNEL_ID) return
  invalidateRequests()
  // 一次替换活动引用与集合，避免同步监听在删除中间态读取不存在的配置。
  aiChannels.value = {
    activeId: aiChannels.value.activeId === id ? OFFICIAL_AI_CHANNEL_ID : aiChannels.value.activeId,
    channels: aiChannels.value.channels.filter((item) => item.id !== id),
  }
  deleteChannelId.value = undefined
}
function openAiSettings() {
  section.value = 'ai'
  void nextTick(() => content.value?.querySelector<HTMLInputElement>('#ai-model')?.focus())
}
defineExpose({ openAiSettings })
async function testConnection() {
  if (saving.value) return
  const snapshot = clone(aiConfig.value)
  const current = ++testGeneration
  connectionTest.value = { pending: true }
  try {
    const result = await desktop.testAiConnection(snapshot)
    if (current === testGeneration && JSON.stringify(aiConfig.value) === JSON.stringify(snapshot))
      connectionTest.value = { pending: false, success: result.translatedText }
  } catch (reason) {
    if (current === testGeneration && JSON.stringify(aiConfig.value) === JSON.stringify(snapshot))
      connectionTest.value = { pending: false, error: errorMessage(reason) }
  } finally {
    if (current === testGeneration && connectionTest.value) connectionTest.value.pending = false
  }
}
async function invalid(messageKey: string, target: Section, selector: string) {
  error.value = t(messageKey)
  section.value = target
  await nextTick()
  content.value?.querySelector<HTMLElement>(selector)?.focus()
}
async function save() {
  error.value = ''
  message.value = ''
  if (!draft.value.hotkeyShowWindow.trim() || !draft.value.hotkeyClipboardTranslate.trim()) {
    await invalid(
      'shortcutRequired',
      'desktop',
      !draft.value.hotkeyShowWindow.trim() ? '#show-window-hotkey' : '#clipboard-hotkey',
    )
    return
  }
  saving.value = true
  invalidateRequests()
  closeDialogs()
  try {
    const saved = await desktop.saveSettings(clone(draft.value), clone(aiChannels.value))
    emit('saved', saved)
    await nextTick()
    message.value = t('saved')
  } catch (reason) {
    error.value = errorMessage(reason)
  } finally {
    saving.value = false
  }
}
</script>
<template>
  <section class="page settings-page">
    <header class="page-heading">
      <h1>{{ t('settings') }}</h1>
      <span v-if="changed" class="unsaved-badge">{{ t('unsaved') }}</span>
    </header>
    <nav class="settings-sections" :aria-label="t('settings')">
      <button
        v-for="item in ['preferences', 'ai', 'desktop'] as const"
        :key="item"
        type="button"
        :class="{ active: section === item }"
        :aria-pressed="section === item"
        @click="section = item"
      >
        {{ t(item) }}
      </button>
    </nav>
    <form class="settings-form" novalidate @submit.prevent="save">
      <fieldset ref="content" :disabled="saving" class="settings-scroll">
        <div v-show="section === 'preferences'" class="settings-section">
          <section class="settings-card card">
            <h2>{{ t('general') }}</h2>
            <div class="form-grid">
              <label class="field" for="settings-locale"
                ><span>{{ t('locale') }}</span
                ><AppSelect
                  id="settings-locale"
                  v-model="draft.locale"
                  :label="t('locale')"
                  :options="[
                    { value: 'zh_CN', label: '简体中文' },
                    { value: 'en_US', label: 'English' },
                  ]"
                  :disabled="saving" /></label
              ><label class="toggle-label"
                ><input v-model="draft.alwaysOnTop" type="checkbox" role="switch" /><span>{{
                  t('alwaysOnTop')
                }}</span></label
              >
            </div>
          </section>
        </div>
        <section v-show="section === 'desktop'" class="settings-card card">
          <h2>{{ t('desktop') }}</h2>
          <label class="toggle-label"
            ><input v-model="draft.minimizeToTray" type="checkbox" role="switch" /><span>{{
              t('minimizeToTray')
            }}</span></label
          >
          <div class="form-grid desktop-shortcuts">
            <div class="field">
              <label for="show-window-hotkey">{{ t('showWindowHotkey') }}</label>
              <ShortcutRecorder
                id="show-window-hotkey"
                :key="editorVersion"
                v-model="draft.hotkeyShowWindow"
                :label="t('showWindowHotkey')"
                :disabled="saving"
              />
            </div>
            <div class="field">
              <label for="clipboard-hotkey">{{ t('clipboardHotkey') }}</label>
              <ShortcutRecorder
                id="clipboard-hotkey"
                :key="editorVersion"
                v-model="draft.hotkeyClipboardTranslate"
                :label="t('clipboardHotkey')"
                :disabled="saving"
              />
            </div>
          </div>
          <p id="hotkey-hint" class="helper muted">{{ t('hotkeyHint') }}</p>
        </section>
        <section v-show="section === 'ai'" class="settings-card card ai-card">
          <AiConfigForm
            :key="editorVersion"
            :config="aiConfig"
            :channels="aiChannels"
            :disabled="saving"
            :test="connectionTest"
            :models="models.state"
            @update="replaceConfig"
            @test="testConnection"
            @models="models.load"
            @channel="selectChannel"
            @add="openNameDialog()"
            @rename="openNameDialog(true)"
            @delete="requestDelete"
          />
          <p class="helper muted security-note">{{ t('securityHint') }}</p>
        </section>
        <details class="data-info">
          <summary>{{ t('dataDirectory') }}</summary>
          <code>{{ data.dataDir }}</code>
        </details>
      </fieldset>
      <footer class="settings-footer">
        <div class="settings-feedback">
          <Notice v-if="error" kind="error">{{ t('saveError') }} · {{ failure(error) }}</Notice
          ><Notice v-else-if="message" kind="success">{{ message }}</Notice>
        </div>
        <div class="settings-actions">
          <button class="button subtle" type="button" :disabled="saving" @click="reset">
            {{ t('cancel') }}</button
          ><button class="button primary" type="submit" :disabled="saving || !changed">
            <span v-if="saving" class="spinner"></span><AppIcon v-else name="check" :size="18" />{{
              saving ? t('saving') : t('saveSettings')
            }}
          </button>
        </div>
      </footer>
    </form>
    <ChannelNameDialog
      v-if="nameDialog"
      :title="t(nameDialog.id ? 'renameAiChannel' : 'addAiChannel')"
      :initial-name="nameDialog.initialName"
      :error="nameDialog.error ? t(nameDialog.error) : ''"
      @cancel="nameDialog = undefined"
      @confirm="confirmName"
    />
    <Teleport to="body">
      <ConfirmAction
        v-if="deleteChannelId"
        :message="t('deleteAiChannelConfirm')"
        :busy="saving"
        @cancel="deleteChannelId = undefined"
        @confirm="confirmDelete"
      />
    </Teleport>
  </section>
</template>
