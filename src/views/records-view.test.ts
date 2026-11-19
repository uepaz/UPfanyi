// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest'
import { desktop } from '../services/desktop'
import { mount, flushPromises } from '@vue/test-utils'
import HistoryView from './HistoryView.vue'
import FavoritesView from './FavoritesView.vue'
import { track, button, favorite } from '../test/support'

vi.mock('../services/desktop', () => ({
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
      'onClipboard',
      'onNavigate',
    ].map((key) => [key, vi.fn()]),
  ),
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

describe('record interactions', () => {
  it('allows all-record operations when a search has no matches', async () => {
    vi.mocked(desktop.listHistory).mockResolvedValue({ items: [], total: 0 })
    vi.mocked(desktop.listFavorites).mockResolvedValue({ items: [], total: 0 })
    vi.mocked(desktop.exportFavorites).mockResolvedValue('test-only/favorites.txt')
    const history = track(mount(HistoryView))
    const favorites = track(mount(FavoritesView))
    await flushPromises()
    await button(history, '清空历史').trigger('click')
    await button(history, '删除').trigger('click')
    await button(favorites, '导出词本').trigger('click')
    await flushPromises()
    expect(desktop.clearHistory).toHaveBeenCalledTimes(1)
    expect(desktop.exportFavorites).toHaveBeenCalledTimes(1)
    expect(favorites.get('.notice.success').text()).toContain('test-only/favorites.txt')
  })
  it('preserves note drafts when switching records and reactivating the page', async () => {
    const other = { ...favorite, id: 2, word: 'two', note: 'other note' }
    vi.mocked(desktop.listFavorites).mockResolvedValue({ items: [favorite, other], total: 2 })
    const wrapper = track(mount(FavoritesView))
    await flushPromises()
    await wrapper.findAll('.record-item')[0]!.trigger('click')
    await wrapper.get('#favorite-note').setValue('draft note')
    await wrapper.findAll('.record-item')[1]!.trigger('click')
    expect((wrapper.get('#favorite-note').element as HTMLTextAreaElement).value).toBe('other note')
    await wrapper.findAll('.record-item')[0]!.trigger('click')
    await wrapper.setProps({ active: false })
    await wrapper.setProps({ active: true })
    await flushPromises()
    expect((wrapper.get('#favorite-note').element as HTMLTextAreaElement).value).toBe('draft note')
    expect(wrapper.findAll('.record-item')[0]!.attributes('aria-pressed')).toBe('true')
    expect(desktop.saveFavorite).not.toHaveBeenCalled()
    await wrapper.get('[aria-label="取消备注修改"]').trigger('click')
    expect((wrapper.get('#favorite-note').element as HTMLTextAreaElement).value).toBe('original')
  })
  it('passes an empty string when the user explicitly clears a note', async () => {
    vi.mocked(desktop.saveFavorite).mockResolvedValue({ ...favorite, note: null })
    const wrapper = track(mount(FavoritesView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    await wrapper.get('#favorite-note').setValue('')
    await button(wrapper, '保存备注').trigger('click')
    await flushPromises()
    expect(desktop.saveFavorite).toHaveBeenCalledExactlyOnceWith('one', '一', '')
  })
  it('clears history details after confirmed deletion', async () => {
    const wrapper = track(mount(HistoryView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    vi.mocked(desktop.listHistory).mockResolvedValue({ items: [], total: 0 })
    await wrapper.get('[aria-label="删除此记录"]').trigger('click')
    await button(wrapper, '删除').trigger('click')
    await flushPromises()
    expect(desktop.deleteHistory).toHaveBeenCalledExactlyOnceWith(1)
    expect(wrapper.find('.detail-text').exists()).toBe(false)
    expect(wrapper.text()).toContain('选择一条记录')
  })
  it('retains history details and displays deletion failure', async () => {
    vi.mocked(desktop.deleteHistory).mockRejectedValue(new Error('database busy'))
    const wrapper = track(mount(HistoryView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    await wrapper.get('[aria-label="删除此记录"]').trigger('click')
    await button(wrapper, '删除').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('database busy')
    expect(wrapper.find('.detail-text').exists()).toBe(true)
  })
  it('does not clear history without confirmation', async () => {
    const wrapper = track(mount(HistoryView))
    await flushPromises()
    await button(wrapper, '清空历史').trigger('click')
    await button(wrapper, '取消').trigger('click')
    expect(desktop.clearHistory).not.toHaveBeenCalled()
  })
  it('passes original source and languages when reusing history', async () => {
    const wrapper = track(mount(HistoryView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    expect(wrapper.get('.model-badge').text()).toBe('fixture-model')
    await button(wrapper, '继续翻译').trigger('click')
    expect(wrapper.emitted('reuse')?.[0]).toEqual([
      { text: 'one', sourceLang: 'en', targetLang: 'zh' },
    ])
  })
  it('preserves unsaved favorite notes when persistence fails', async () => {
    vi.mocked(desktop.saveFavorite).mockRejectedValue(new Error('cannot write'))
    const wrapper = track(mount(FavoritesView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    await wrapper.get('#favorite-note').setValue('new note')
    await button(wrapper, '保存备注').trigger('click')
    await flushPromises()
    expect(desktop.saveFavorite).toHaveBeenCalledExactlyOnceWith('one', '一', 'new note')
    expect(wrapper.get('[role="alert"]').text()).toContain('cannot write')
    expect((wrapper.get('#favorite-note').element as HTMLTextAreaElement).value).toBe('new note')
  })
  it('clears favorite details after deletion', async () => {
    const wrapper = track(mount(FavoritesView))
    await flushPromises()
    await wrapper.get('.record-item').trigger('click')
    vi.mocked(desktop.listFavorites).mockResolvedValue({ items: [], total: 0 })
    await wrapper.get('[aria-label="删除此记录"]').trigger('click')
    await button(wrapper, '删除').trigger('click')
    await flushPromises()
    expect(desktop.deleteFavorite).toHaveBeenCalledExactlyOnceWith(1)
    expect(wrapper.find('#favorite-note').exists()).toBe(false)
  })
  it('does not report a cancelled export as success', async () => {
    vi.mocked(desktop.exportFavorites).mockResolvedValue(null)
    const wrapper = track(mount(FavoritesView))
    await flushPromises()
    await button(wrapper, '导出词本').trigger('click')
    await flushPromises()
    expect(wrapper.find('.notice.success').exists()).toBe(false)
  })
})
