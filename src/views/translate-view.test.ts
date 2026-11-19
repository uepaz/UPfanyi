// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { desktop } from '../services/desktop'
import { mount, flushPromises } from '@vue/test-utils'
import TranslateView from './TranslateView.vue'
import { setLocale } from '../i18n'
import { bootstrap, track, translation, chooseSelect, deferred } from '../test/support'
import { mockTargetLanguage } from '../test/targetLanguage'

vi.mock('../services/desktop', () => ({
  targetLanguage: { get: vi.fn(), set: vi.fn(), onChange: vi.fn() },
  desktop: Object.fromEntries(
    [
      'bootstrap',
      'translate',
      'testAiConnection',
      'saveSettings',
      'listHistory',
      'deleteHistory',
      'clearHistory',
      'listFavorites',
      'saveFavorite',
      'deleteFavorite',
      'exportFavorites',
      'copyText',
      'onNavigate',
    ].map((key) => [key, vi.fn()]),
  ),
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

beforeEach(() => mockTargetLanguage('en'))

describe('translation view', () => {
  it.each([
    ['AI 流式响应格式无效', 'invalid translation stream'],
    ['AI 服务返回流式错误，请检查配置或重试', 'streaming error'],
    ['AI 请求超时，请重试', 'timed out'],
    ['AI 服务拒绝了此次翻译请求', 'declined'],
  ])('shows a localized stream error for %s', async (serverError, expected) => {
    setLocale('en_US')
    vi.mocked(desktop.translate).mockRejectedValueOnce(serverError)
    const view = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await view.get('textarea').setValue('one')
    await view.get('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(view.get('[role="alert"]').text()).toContain(expected)
  })
  it('displays incoming text before completion and enables copy and favorite only for the final translation', async () => {
    const reply = deferred<ReturnType<typeof translation>>()
    vi.mocked(desktop.translate).mockReturnValueOnce(reply.promise)
    const view = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await view.get('textarea').setValue('one')
    await view.get('textarea').trigger('keydown', { key: 'Enter' })
    const progress = vi.mocked(desktop.translate).mock.calls[0]![1]
    progress('第')
    progress('一段')
    await flushPromises()
    expect(view.get('.result-panel').attributes('aria-busy')).toBe('true')
    expect(view.get('.result-content').text()).toBe('第一段')
    expect(view.get('[aria-label="复制 译文"]').attributes('disabled')).toBeDefined()
    expect(view.get('[aria-label="收藏"]').attributes('disabled')).toBeDefined()
    reply.resolve(translation('one', '第一段译文'))
    await flushPromises()
    progress('late')
    await flushPromises()
    expect(view.get('.result-content').text()).toBe('第一段译文')
    await view.get('[aria-label="复制 译文"]').trigger('click')
    expect(desktop.copyText).toHaveBeenCalledExactlyOnceWith('第一段译文')
  })
  it('keeps auto detection exclusive to source and sends the languages selected after swapping', async () => {
    const wrapper = track(mount(TranslateView, { props: { data: bootstrap() } }))
    expect(wrapper.get('#translate-source-lang').attributes('aria-label')).toBe('源语言')
    expect(wrapper.get('#translate-source-lang').attributes('data-value')).toBe('auto')
    await wrapper.get('[aria-label="交换语言"]').trigger('click')
    expect(wrapper.get('[role="alert"]').text()).toContain('请选择明确的源语言后再交换。')
    expect(wrapper.get('#translate-source-lang').attributes('data-value')).toBe('auto')
    await wrapper.get('#translate-target-lang').trigger('click')
    expect(
      document
        .getElementById('translate-target-lang-listbox')
        ?.querySelector('[data-value="auto"]'),
    ).toBeNull()
    await wrapper.get('#translate-target-lang').trigger('keydown', { key: 'Escape' })
    await chooseSelect(wrapper, 'translate-source-lang', 'ja')
    await chooseSelect(wrapper, 'translate-target-lang', 'fr')
    await wrapper.get('[aria-label="交换语言"]').trigger('click')
    expect(wrapper.get('#translate-source-lang').attributes('data-value')).toBe('fr')
    expect(wrapper.get('#translate-target-lang').attributes('data-value')).toBe('ja')
    await wrapper.get('textarea').setValue('one')
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(desktop.translate).toHaveBeenCalledExactlyOnceWith(
      {
        text: 'one',
        sourceLang: 'fr',
        targetLang: 'ja',
      },
      expect.any(Function),
    )
  })
  it('handles Enter and Shift+Enter independently', async () => {
    const wrapper = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await wrapper.get('textarea').setValue('one')
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter', shiftKey: true })
    expect(desktop.translate).not.toHaveBeenCalled()
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(desktop.translate).toHaveBeenCalledTimes(1)
  })
  it('saves the displayed result pair after the input has changed', async () => {
    const wrapper = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await wrapper.get('textarea').setValue('one')
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    await wrapper.get('textarea').setValue('changed input')
    await wrapper.get('[aria-label="收藏"]').trigger('click')
    await flushPromises()
    expect(desktop.saveFavorite).toHaveBeenCalledExactlyOnceWith('one', '一', null)
    expect(wrapper.get('.paired-source').text()).toContain('one')
  })
  it('reports copy failures without claiming success', async () => {
    vi.mocked(desktop.copyText).mockRejectedValue(new Error('clipboard locked'))
    const wrapper = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await wrapper.get('textarea').setValue('one')
    await wrapper.get('[aria-label="复制 原文"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('clipboard locked')
    expect(wrapper.text()).not.toContain('已复制')
  })
  it('keeps translated text visible when history persistence returns a warning', async () => {
    vi.mocked(desktop.translate).mockResolvedValue({
      ...translation(),
      model: 'actual-response-model',
      warning: 'history write failed',
    })
    const wrapper = track(mount(TranslateView, { props: { data: bootstrap() } }))
    await wrapper.get('textarea').setValue('one')
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(wrapper.get('.result-content').text()).toContain('一')
    expect(wrapper.get('.model-badge').text()).toBe('actual-response-model')
    expect(wrapper.get('.notice.warning').text()).toBe('history write failed')
  })
})
