// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import AiConfigForm from './AiConfigForm.vue'
import { setLocale } from '../i18n'
import { chooseSelect, deferred, fixtureChannelId } from '../test/support'
import { desktop } from '../services/desktop'
import { flushPromises } from '@vue/test-utils'
import { OPENAI_ENDPOINT, UPFANYI_AI_ENDPOINT } from '../services/aiConfig'
import type { AiChannels, AiConfig } from '../types/desktop'

vi.mock('../services/desktop', () => ({
  desktop: Object.fromEntries(
    [
      'bootstrap',
      'translate',
      'onNavigate',
      'listHistory',
      'listFavorites',
      'saveFavorite',
      'openOfficialWebsite',
    ].map((key) => [key, vi.fn()]),
  ),
}))

let wrapper: VueWrapper
function channelsWithActive(activeId: string): AiChannels {
  return { ...(wrapper.props() as { channels: AiChannels }).channels, activeId }
}
function render(overrides: Partial<AiConfig> = {}, official = false) {
  const config: AiConfig = {
    protocol: 'openai',
    endpoint: '',
    model: '',
    apiKey: '',
    systemPrompt: '',
    ...overrides,
  }
  const channels: AiChannels = {
    activeId: official ? 'upfanyi' : fixtureChannelId,
    channels: [
      {
        id: 'upfanyi',
        name: 'UP翻译官方 AI',
        config: { ...config, endpoint: UPFANYI_AI_ENDPOINT },
      },
      { id: fixtureChannelId, name: 'My service', config },
    ],
  }
  wrapper = mount(AiConfigForm, { props: { config, channels } })
  return config
}
beforeEach(() => setLocale('zh_CN'))
afterEach(() => wrapper.unmount())

describe('explicit AI configuration', () => {
  it('places the add action between official and named channels without changing selection', async () => {
    render({ endpoint: UPFANYI_AI_ENDPOINT })
    expect(wrapper.find('#ai-endpoint').exists()).toBe(true)
    expect(wrapper.find('.official-website-button').exists()).toBe(false)
    await wrapper.get('#ai-channel').trigger('click')
    const options = Array.from(document.querySelectorAll('#ai-channel-listbox [role="option"]'))
    expect(options.map((item) => item.textContent)).toEqual([
      'UP翻译官方 AI',
      '添加渠道',
      'My service',
    ])
    const action = options[1] as HTMLElement
    expect(action.hasAttribute('aria-selected')).toBe(false)
    action.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('add')).toHaveLength(1)
    expect(wrapper.emitted('channel')).toBeUndefined()
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(wrapper.get('.channel-actions').text()).toContain('重命名渠道')
    await wrapper.get('#ai-channel').trigger('keydown', { key: 'Home' })
    await wrapper.get('#ai-channel').trigger('keydown', { key: 'ArrowDown' })
    await wrapper.get('#ai-channel').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('add')).toHaveLength(2)
    expect(wrapper.emitted('channel')).toBeUndefined()
  })
  it('renders empty required values without emitting initialized defaults', () => {
    render()
    expect((wrapper.get('#ai-endpoint').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('')
    expect(wrapper.get('.ai-state').text()).toContain('待配置')
    expect(wrapper.emitted('update')).toBeUndefined()
  })
  it('keeps protocol and API controls separate without editing the draft from the sole protocol option', async () => {
    render({ endpoint: 'http://localhost:11434', model: 'fixture-model' })
    expect(wrapper.get('#ai-protocol').text()).toBe('OpenAI 兼容')
    await wrapper.get('#ai-protocol').trigger('click')
    expect(
      Array.from(document.querySelectorAll('#ai-protocol-listbox [role="option"]')).map((item) =>
        item.getAttribute('data-value'),
      ),
    ).toEqual(['openai'])
    document.querySelector<HTMLElement>('#ai-protocol-listbox [role="option"]')!.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.get('#ai-interface').attributes('data-value')).toBe('openai')
    expect(wrapper.get('.ai-state').text()).toBe('已配置')
  })
  it('places the editable model below the secret and exposes only valid interfaces', async () => {
    render({ model: 'manual-model' })
    const fields = wrapper
      .findAll('.ai-fields input, .ai-fields [role="combobox"]')
      .map((item) => item.attributes('id'))
    expect(fields).toEqual([
      'ai-channel',
      'ai-endpoint',
      'ai-protocol',
      'ai-interface',
      'ai-api-key',
      'ai-model',
    ])
    await wrapper.get('#ai-interface').trigger('click')
    expect(
      Array.from(document.querySelectorAll('#ai-interface-listbox [role="option"]')).map(
        (item) => item.textContent,
      ),
    ).toEqual(['Chat Completions', 'Responses'])
    await wrapper.get('#ai-interface').trigger('keydown', { key: 'Escape' })
  })
  it('selects Responses without mutating the original draft and exposes its family', async () => {
    const config = render({ endpoint: OPENAI_ENDPOINT })
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    const updated = wrapper.emitted('update')?.[0]?.[0] as AiConfig
    expect(updated).toEqual({
      ...config,
      protocol: 'openai-responses',
      endpoint: OPENAI_ENDPOINT,
    })
    expect(config.protocol).toBe('openai')
    await wrapper.setProps({ config: updated })
    expect(wrapper.get('#ai-protocol').attributes('data-value')).toBe('openai')
    expect(wrapper.get('#ai-interface').attributes('data-value')).toBe('openai-responses')
  })
  it('describes the domain-only service address in both interface languages', async () => {
    render({ endpoint: 'relay.example.com', apiKey: '', model: 'fixture-model' })
    expect(wrapper.get('label[for="ai-endpoint"]').text()).toContain('服务地址')
    expect(wrapper.get('#ai-endpoint').attributes('placeholder')).toContain('relay.example.com')
    expect(wrapper.get('#ai-endpoint-hint').text()).toContain('默认 HTTPS')
    expect(wrapper.get('#ai-endpoint-hint').text()).toContain('/v1 不会重复')
    expect(wrapper.get('.ai-state').text()).toBe('已配置')
    setLocale('en_US')
    await wrapper.vm.$nextTick()
    expect(wrapper.get('label[for="ai-endpoint"]').text()).toContain('Service address')
    expect(wrapper.get('#ai-endpoint-hint').text()).toContain('HTTPS is assumed')
  })
  it('retains a domain-only base address on interface switches', async () => {
    const config = render({ endpoint: 'relay.example.com:8443/v1?tenant=test' })
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    const updated = wrapper.emitted('update')?.[0]?.[0] as AiConfig
    expect(updated).toEqual({ ...config, protocol: 'openai-responses' })
    await wrapper.setProps({ config: updated })
    await chooseSelect(wrapper, 'ai-interface', 'openai')
    expect(wrapper.emitted('update')?.[1]?.[0]).toEqual(config)
  })
  it('provides styled searchable choices while allowing an unlisted model and explicit fetching', async () => {
    const config = render({ model: 'original' })
    await wrapper.setProps({
      models: { pending: false, error: '', items: ['model-a', 'model-b'], loaded: true },
    })
    expect(wrapper.get('#ai-model').attributes('role')).toBe('combobox')
    expect(wrapper.find('datalist').exists()).toBe(false)
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('original')
    expect(wrapper.emitted('update')).toBeUndefined()
    await wrapper.get('.app-combobox-toggle').trigger('click')
    expect(
      Array.from(document.querySelectorAll('#ai-model-listbox [role="option"]')).map((item) =>
        item.getAttribute('data-value'),
      ),
    ).toEqual(['model-a', 'model-b'])
    document.querySelector<HTMLElement>('#ai-model-listbox [data-value="model-b"]')!.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update')?.[0]?.[0]).toEqual({ ...config, model: 'model-b' })
    expect(document.getElementById('ai-model-listbox')).toBeNull()
    await wrapper.get('#ai-model').setValue('unlisted-model')
    expect(wrapper.emitted('update')?.[1]?.[0]).toEqual({ ...config, model: 'unlisted-model' })
    await wrapper
      .findAll('button')
      .find((item) => item.text() === '获取模型')!
      .trigger('click')
    expect(wrapper.emitted('models')).toHaveLength(1)
  })
  it('emits explicit empty credentials and only toggles display on reveal', async () => {
    const config = render({ apiKey: 'fictional' })
    await wrapper.get('[aria-label="显示密钥"]').trigger('click')
    expect(wrapper.get('#ai-api-key').attributes('type')).toBe('text')
    expect(wrapper.emitted('update')).toBeUndefined()
    await wrapper.get('[aria-label="隐藏密钥"]').trigger('click')
    expect(wrapper.get('#ai-api-key').attributes('type')).toBe('password')
    await wrapper.get('#ai-api-key').setValue('')
    expect(wrapper.emitted('update')?.[0]?.[0]).toEqual({ ...config, apiKey: '' })
  })
  it('emits channel identity without replacing or clearing another channel configuration', async () => {
    const original = render({
      protocol: 'openai',
      endpoint: 'http://localhost:11434',
      apiKey: 'other-service-key',
      model: 'other-model',
      systemPrompt: 'Keep this prompt',
    })
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    expect(wrapper.emitted('channel')?.[0]).toEqual(['upfanyi'])
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(original.apiKey).toBe('other-service-key')
    expect(wrapper.emitted('models')).toBeUndefined()
    expect(wrapper.emitted('test')).toBeUndefined()
  })
  it('keeps both official interfaces and shows the website instead of the address', async () => {
    const original = render(
      {
        endpoint: UPFANYI_AI_ENDPOINT,
        apiKey: 'official-test-key',
        model: 'official-model',
      },
      true,
    )
    expect(wrapper.find('#ai-endpoint').exists()).toBe(false)
    expect(wrapper.get('.official-website-button').text()).toBe('访问官方网站')
    await wrapper.get('#ai-protocol').trigger('click')
    expect(
      Array.from(document.querySelectorAll('#ai-protocol-listbox [role="option"]')).map((item) =>
        item.getAttribute('data-value'),
      ),
    ).toEqual(['openai'])
    await wrapper.get('#ai-protocol').trigger('keydown', { key: 'Escape' })
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    const updated = wrapper.emitted('update')?.[0]?.[0] as AiConfig
    expect(updated).toEqual({ ...original, protocol: 'openai-responses' })
    await wrapper.setProps({ config: updated })
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    await wrapper.setProps({
      channels: channelsWithActive(fixtureChannelId),
    })
    expect(wrapper.find('.official-website-button').exists()).toBe(false)
    expect(wrapper.find('#ai-endpoint').exists()).toBe(true)
  })
  it('shows the official channel and the live setup status beside the heading in both languages', async () => {
    const config = render({ protocol: 'openai-responses', endpoint: UPFANYI_AI_ENDPOINT }, true)
    const heading = wrapper.get('header')
    const status = heading.get('h2 + [role="status"]')
    expect(heading.get('h2').text()).toBe('AI 翻译')
    expect(status.text()).toContain('需要填写 模型')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.get('#ai-channel').text()).toBe('UP翻译官方 AI')
    expect(wrapper.get('#ai-channel').attributes('aria-describedby')).toBeUndefined()
    expect(wrapper.get('#ai-model').attributes('aria-describedby')).toBe('ai-model-status')
    expect(wrapper.get('#ai-model-status').attributes('aria-live')).toBe('polite')
    await wrapper.setProps({ config: { ...config, model: 'fixture-model' } })
    expect(status.text()).toBe('已配置')
    setLocale('en_US')
    await wrapper.vm.$nextTick()
    expect(wrapper.get('label[for="ai-channel"]').text()).toContain('AI channel')
    expect(wrapper.get('#ai-channel').text()).toBe('UPfanyi official AI')
    expect(heading.get('h2').text()).toBe('AI translation')
    expect(status.text()).toBe('Configured')
    await wrapper.setProps({ config })
    expect(status.text()).toContain('Required fields: Model')
  })
  it('opens the fixed website only on click and displays friendly bilingual failures without changing configuration', async () => {
    const original = render(
      {
        endpoint: UPFANYI_AI_ENDPOINT,
        apiKey: 'official-key',
        model: 'official-model',
      },
      true,
    )
    const pending = deferred<void>()
    vi.mocked(desktop.openOfficialWebsite).mockReturnValueOnce(pending.promise)
    expect(desktop.openOfficialWebsite).not.toHaveBeenCalled()
    await wrapper.get('.official-website-button').trigger('click')
    expect(desktop.openOfficialWebsite).toHaveBeenCalledExactlyOnceWith()
    expect(wrapper.get('.official-website-button').attributes('disabled')).toBeDefined()
    await wrapper.get('.official-website-button').trigger('click')
    expect(desktop.openOfficialWebsite).toHaveBeenCalledTimes(1)
    pending.reject(new Error('OFFICIAL_WEBSITE_OPEN_FAILED'))
    await flushPromises()
    expect(wrapper.get('.official-website [role="alert"]').text()).toBe(
      '无法打开官方网站，请稍后重试。',
    )
    expect(wrapper.get('.official-website-button').attributes('disabled')).toBeUndefined()
    setLocale('en_US')
    await wrapper.vm.$nextTick()
    expect(wrapper.get('.official-website-button').text()).toBe('Visit official website')
    expect(wrapper.get('.official-website [role="alert"]').text()).toContain(
      'Unable to open the official website',
    )
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.emitted('models')).toBeUndefined()
    expect(wrapper.emitted('test')).toBeUndefined()
    expect(original.apiKey).toBe('official-key')
    expect(original.model).toBe('official-model')
    vi.mocked(desktop.openOfficialWebsite).mockResolvedValueOnce()
    await wrapper.get('.official-website-button').trigger('click')
    await flushPromises()
    expect(wrapper.find('.official-website [role="alert"]').exists()).toBe(false)
  })
  it('ignores an older website failure after leaving and returning to official channel', async () => {
    render({ endpoint: UPFANYI_AI_ENDPOINT }, true)
    const old = deferred<void>()
    const current = deferred<void>()
    vi.mocked(desktop.openOfficialWebsite)
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(current.promise)
    await wrapper.get('.official-website-button').trigger('click')
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    await wrapper.setProps({
      channels: channelsWithActive(fixtureChannelId),
    })
    expect(wrapper.find('.official-website-button').exists()).toBe(false)
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    await wrapper.setProps({ channels: channelsWithActive('upfanyi') })
    await wrapper.get('.official-website-button').trigger('click')
    old.reject(new Error('old-failure'))
    await flushPromises()
    expect(wrapper.find('.official-website [role="alert"]').exists()).toBe(false)
    expect(wrapper.get('.official-website-button').attributes('disabled')).toBeDefined()
    current.resolve()
    await flushPromises()
    expect(wrapper.get('.official-website-button').attributes('disabled')).toBeUndefined()
  })
  it('ignores website completion after unmount and respects saving disablement', async () => {
    render({ endpoint: UPFANYI_AI_ENDPOINT }, true)
    await wrapper.setProps({ disabled: true })
    await wrapper.get('.official-website-button').trigger('click')
    expect(desktop.openOfficialWebsite).not.toHaveBeenCalled()
    expect(wrapper.get('#ai-channel').attributes('disabled')).toBeDefined()
    await wrapper.setProps({ disabled: false })
    const pending = deferred<void>()
    vi.mocked(desktop.openOfficialWebsite).mockReturnValueOnce(pending.promise)
    await wrapper.get('.official-website-button').trigger('click')
    wrapper.unmount()
    pending.reject(new Error('unmounted'))
    await flushPromises()
    expect(document.querySelector('.official-website')).toBeNull()
  })
})
