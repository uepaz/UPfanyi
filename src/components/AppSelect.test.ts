// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import AppSelect from './AppSelect.vue'

let wrapper: VueWrapper
function render() {
  wrapper = mount(AppSelect, {
    attachTo: document.body,
    props: {
      id: 'test-select',
      modelValue: 'first',
      label: '协议',
      options: [
        { value: 'first', label: 'First' },
        { value: 'disabled', label: 'Disabled', disabled: true },
        { value: 'last', label: 'Last' },
      ],
    },
  })
  return wrapper.get('[role="combobox"]')
}
function list() {
  return document.getElementById('test-select-listbox')
}
function active() {
  return wrapper.get('[role="combobox"]').attributes('aria-activedescendant')
}
afterEach(() => {
  wrapper.unmount()
  vi.restoreAllMocks()
})

describe('application select', () => {
  it('shows a labeled listbox with selected and disabled states and selects by mouse', async () => {
    const trigger = render()
    expect(trigger.attributes('aria-label')).toBe('协议')
    expect(trigger.attributes('aria-expanded')).toBe('false')
    await trigger.trigger('click')
    expect(trigger.attributes('aria-expanded')).toBe('true')
    expect(list()?.getAttribute('role')).toBe('listbox')
    expect(list()?.querySelector('[data-value="first"]')?.getAttribute('aria-selected')).toBe(
      'true',
    )
    expect(list()?.querySelector('[data-value="disabled"]')?.getAttribute('aria-disabled')).toBe(
      'true',
    )
    list()?.querySelector<HTMLElement>('[data-value="disabled"]')?.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect(list()).not.toBeNull()
    list()?.querySelector<HTMLElement>('[data-value="last"]')?.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['last'])
    expect(list()).toBeNull()
    expect(document.activeElement).toBe(trigger.element)
  })
  it('uses arrows, Home/End and Enter/Space without selecting disabled options', async () => {
    const trigger = render()
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    expect(active()).toBe('test-select-option-0')
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    expect(active()).toBe('test-select-option-2')
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    expect(active()).toBe('test-select-option-0')
    await trigger.trigger('keydown', { key: 'ArrowUp' })
    expect(active()).toBe('test-select-option-2')
    await trigger.trigger('keydown', { key: 'Home' })
    expect(active()).toBe('test-select-option-0')
    await trigger.trigger('keydown', { key: 'End' })
    expect(active()).toBe('test-select-option-2')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await trigger.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['last'])
    await trigger.trigger('keydown', { key: 'Home' })
    expect(active()).toBe('test-select-option-0')
    await trigger.trigger('keydown', { key: ' ' })
    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual(['first'])
    await trigger.trigger('keydown', { key: 'End' })
    expect(active()).toBe('test-select-option-2')
  })
  it('closes on Escape, Tab, focus leaving or outside click without committing', async () => {
    const trigger = render()
    await trigger.trigger('click')
    await trigger.trigger('keydown', { key: 'Escape' })
    expect(list()).toBeNull()
    await trigger.trigger('click')
    const tab = new KeyboardEvent('keydown', { key: 'Tab', cancelable: true })
    trigger.element.dispatchEvent(tab)
    await wrapper.vm.$nextTick()
    expect(tab.defaultPrevented).toBe(false)
    expect(list()).toBeNull()
    await trigger.trigger('click')
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }))
    await wrapper.vm.$nextTick()
    expect(list()).toBeNull()
    await trigger.trigger('click')
    await trigger.trigger('blur')
    expect(list()).toBeNull()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  })
  it('closes when disabled and releases listeners and teleported content on unmount', async () => {
    const trigger = render()
    await trigger.trigger('click')
    await wrapper.setProps({ disabled: true })
    expect(list()).toBeNull()
    await trigger.trigger('click')
    await trigger.trigger('keydown', { key: 'Enter' })
    expect(list()).toBeNull()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await wrapper.setProps({ disabled: false })
    await trigger.trigger('click')
    const removeDocument = vi.spyOn(document, 'removeEventListener')
    const removeWindow = vi.spyOn(window, 'removeEventListener')
    wrapper.unmount()
    expect(list()).toBeNull()
    expect(removeDocument).toHaveBeenCalledWith('pointerdown', expect.any(Function))
    expect(removeWindow).toHaveBeenCalledWith('resize', expect.any(Function))
    expect(removeWindow).toHaveBeenCalledWith('scroll', expect.any(Function), true)
  })
  it('opens above when the lower viewport is constrained and tracks scrolling and resizing', async () => {
    const trigger = render()
    vi.spyOn(trigger.element, 'getBoundingClientRect').mockReturnValue({
      left: 100,
      top: window.innerHeight - 65,
      bottom: window.innerHeight - 20,
      width: 240,
    } as DOMRect)
    await trigger.trigger('click')
    expect(list()?.style.bottom).toBe('71px')
    expect(list()?.style.top).toBe('')
    expect(list()?.style.width).toBe('240px')
    vi.mocked(trigger.element.getBoundingClientRect).mockReturnValue({
      left: 40,
      top: 20,
      bottom: 62,
      width: 220,
    } as DOMRect)
    window.dispatchEvent(new Event('scroll'))
    await wrapper.vm.$nextTick()
    expect(list()?.style.top).toBe('68px')
    expect(list()?.style.bottom).toBe('')
    window.dispatchEvent(new Event('resize'))
    await wrapper.vm.$nextTick()
    expect(list()?.style.width).toBe('220px')
  })
  it('moves away from an option disabled while the menu is open', async () => {
    const trigger = render()
    await trigger.trigger('keydown', { key: 'End' })
    await wrapper.setProps({
      options: [
        { value: 'first', label: 'First' },
        { value: 'last', label: 'Last', disabled: true },
      ],
    })
    expect(active()).toBe('test-select-option-0')
    await trigger.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['first'])
  })
})
