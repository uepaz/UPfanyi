// @vitest-environment jsdom
import { defineComponent, h, ref } from 'vue'
import { mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ShortcutRecorder from './ShortcutRecorder.vue'
import { provideSelectScope } from '../composables/useSelectScope'
import { setLocale } from '../i18n'

let wrapper: VueWrapper
beforeEach(() => setLocale('zh_CN'))
afterEach(() => wrapper?.unmount())
function render(disabled = false) {
  wrapper = mount(ShortcutRecorder, {
    props: { id: 'fixture-hotkey', modelValue: 'Ctrl+Shift+T', label: '显示窗口快捷键', disabled },
    attachTo: document.body,
  })
  return wrapper.get('input')
}
function value() {
  return (wrapper.get('input').element as HTMLInputElement).value
}

describe('shortcut recording', () => {
  it('previews modifiers, emits one physical combo and suppresses defaults without text editing', async () => {
    const input = render()
    expect(input.attributes('readonly')).toBeDefined()
    await input.trigger('keydown', { key: 'z', code: 'KeyZ', ctrlKey: true })
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await input.trigger('click')
    await input.trigger('keydown', { key: 'Control', code: 'ControlLeft', ctrlKey: true })
    expect(value()).toBe('Ctrl+…')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await input.trigger('keyup', { key: 'Control', code: 'ControlLeft' })
    expect(value()).toBe('请按下快捷键…')
    const press = new KeyboardEvent('keydown', {
      key: '+',
      code: 'Equal',
      ctrlKey: true,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    })
    input.element.dispatchEvent(press)
    await wrapper.vm.$nextTick()
    expect(press.defaultPrevented).toBe(true)
    expect(wrapper.emitted('update:modelValue')).toEqual([['Ctrl+Shift+Equal']])
    expect(wrapper.classes()).not.toContain('recording')
    await wrapper.setProps({ modelValue: 'Ctrl+Shift+Equal' })
    expect(value()).toBe('Ctrl+Shift+Equal')
    const paste = new Event('paste', { bubbles: true, cancelable: true })
    input.element.dispatchEvent(paste)
    expect(paste.defaultPrevented).toBe(true)
    expect(wrapper.emitted('update:modelValue')).toHaveLength(1)
  })
  it('can start with keyboard, records Enter once and prevents held Enter from submitting or restarting', async () => {
    const input = render()
    await input.trigger('keydown', { key: ' ', code: 'Space' })
    expect(wrapper.classes()).toContain('recording')
    const enter = new KeyboardEvent('keydown', {
      key: 'Enter',
      code: 'Enter',
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    })
    input.element.dispatchEvent(enter)
    await wrapper.vm.$nextTick()
    expect(enter.defaultPrevented).toBe(true)
    expect(wrapper.emitted('update:modelValue')).toEqual([['Ctrl+Enter']])
    const repeat = new KeyboardEvent('keydown', {
      key: 'Enter',
      code: 'Enter',
      repeat: true,
      bubbles: true,
      cancelable: true,
    })
    input.element.dispatchEvent(repeat)
    expect(repeat.defaultPrevented).toBe(true)
    expect(wrapper.classes()).not.toContain('recording')
  })
  it('ignores repeat and IME, shows bilingual unsupported-key feedback and permits retry', async () => {
    const input = render()
    await input.trigger('click')
    await input.trigger('keydown', { key: 't', code: 'KeyT', repeat: true })
    await input.trigger('keydown', { key: 'Enter', code: 'Enter', isComposing: true })
    await input.trigger('keydown', { key: 'Tab', code: 'Tab', isComposing: true })
    await input.trigger('keydown', { key: 'Escape', code: 'Escape', repeat: true })
    await input.trigger('keydown', { key: 'Process', code: 'KeyT', keyCode: 229 })
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect(wrapper.classes()).toContain('recording')
    await input.trigger('keydown', { key: 'Unidentified', code: 'Unidentified' })
    expect(wrapper.get('small').text()).toContain('暂不支持')
    expect(input.attributes('aria-invalid')).toBe('true')
    setLocale('en_US')
    await wrapper.vm.$nextTick()
    expect(wrapper.get('small').text()).toContain('not supported')
    await input.trigger('keydown', { key: 'F8', code: 'F8', metaKey: true })
    expect(wrapper.emitted('update:modelValue')).toEqual([['Super+F8']])
    await wrapper.setProps({ modelValue: 'Super+F8' })
    expect(value()).toBe('Win+F8')
  })
  it.each(['Escape', 'Tab', 'ShiftTab', 'blur', 'windowBlur'])(
    'cancels %s without submitting a partial shortcut',
    async (exit) => {
      const input = render()
      await input.trigger('click')
      await input.trigger('keydown', { key: 'Control', code: 'ControlLeft', ctrlKey: true })
      if (exit === 'blur') await input.trigger('blur')
      else if (exit === 'windowBlur') window.dispatchEvent(new Event('blur'))
      else {
        const event = new KeyboardEvent('keydown', {
          key: exit === 'ShiftTab' ? 'Tab' : exit,
          code: exit === 'ShiftTab' ? 'Tab' : exit,
          shiftKey: exit === 'ShiftTab',
          bubbles: true,
          cancelable: true,
        })
        input.element.dispatchEvent(event)
        if (exit !== 'Escape') expect(event.defaultPrevented).toBe(false)
      }
      await wrapper.vm.$nextTick()
      expect(wrapper.classes()).not.toContain('recording')
      expect(value()).toBe('Ctrl+Shift+T')
      await input.trigger('keydown', { key: 'z', code: 'KeyZ', ctrlKey: true })
      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    },
  )
  it('stops on disabling, snapshot changes and unmount and removes the window listener', async () => {
    const input = render(true)
    await input.trigger('click')
    await input.trigger('keydown', { key: 'Enter', code: 'Enter' })
    expect(wrapper.classes()).not.toContain('recording')
    await wrapper.setProps({ disabled: false })
    await input.trigger('click')
    await wrapper.setProps({ disabled: true })
    expect(wrapper.classes()).not.toContain('recording')
    await wrapper.setProps({ disabled: false })
    await input.trigger('click')
    await wrapper.setProps({ modelValue: 'Alt+F9' })
    expect(value()).toBe('Alt+F9')
    await input.trigger('click')
    const remove = vi.spyOn(window, 'removeEventListener')
    wrapper.unmount()
    expect(remove).toHaveBeenCalledWith('blur', expect.any(Function))
    remove.mockRestore()
  })
  it('ends capture on a programmatic scope change without relying on browser blur', async () => {
    const page = ref('settings/desktop')
    wrapper = mount(
      defineComponent({
        setup() {
          provideSelectScope(page)
          return () =>
            h(ShortcutRecorder, { id: 'scoped', modelValue: 'Ctrl+T', label: 'Shortcut' })
        },
      }),
    )
    const input = wrapper.get('input')
    await input.trigger('click')
    expect(wrapper.get('.shortcut-recorder').classes()).toContain('recording')
    page.value = 'translation'
    await wrapper.vm.$nextTick()
    await input.trigger('keydown', { key: 'z', code: 'KeyZ', ctrlKey: true })
    expect(wrapper.getComponent(ShortcutRecorder).emitted('update:modelValue')).toBeUndefined()
    expect(wrapper.get('.shortcut-recorder').classes()).not.toContain('recording')
  })
})
