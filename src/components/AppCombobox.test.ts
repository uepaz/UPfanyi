// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import AppCombobox from './AppCombobox.vue'

let wrapper: VueWrapper
function render(modelValue = '', options = ['Model-A', 'model-b', 'other']) {
  wrapper = mount(AppCombobox, {
    attachTo: document.body,
    props: {
      id: 'model-choice',
      modelValue,
      options,
      label: '模型',
      describedBy: 'model-hint',
      placeholder: '可搜索或手填',
      'onUpdate:modelValue': (value: string) => void wrapper.setProps({ modelValue: value }),
    },
  })
  return wrapper.get('input')
}
function list() {
  return document.getElementById('model-choice-listbox')
}
function values() {
  return Array.from(list()?.querySelectorAll('[role="option"]') ?? []).map((option) =>
    option.getAttribute('data-value'),
  )
}
afterEach(() => {
  wrapper?.unmount()
  vi.restoreAllMocks()
})

describe('editable application combobox', () => {
  it('exposes a labeled listbox and filters without replacing arbitrary typed text', async () => {
    const input = render()
    expect(input.attributes('role')).toBe('combobox')
    expect(input.attributes('aria-label')).toBe('模型')
    expect(input.attributes('aria-describedby')).toBe('model-hint')
    expect(input.attributes('placeholder')).toBe('可搜索或手填')
    await input.trigger('focus')
    expect(list()?.getAttribute('role')).toBe('listbox')
    expect(list()?.getAttribute('aria-label')).toBe('模型')
    expect(values()).toEqual(['Model-A', 'model-b', 'other'])
    await input.setValue('MODEL')
    expect(values()).toEqual(['Model-A', 'model-b'])
    expect((input.element as HTMLInputElement).value).toBe('MODEL')
    await input.setValue('private-unlisted-model')
    expect(list()).toBeNull()
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['private-unlisted-model'])
    expect((input.element as HTMLInputElement).value).toBe('private-unlisted-model')
  })
  it('shows all candidates from the arrow even when the current model does not match', async () => {
    const input = render('unlisted')
    await input.trigger('focus')
    expect(list()).toBeNull()
    await wrapper.get('.app-combobox-toggle').trigger('click')
    expect(values()).toEqual(['Model-A', 'model-b', 'other'])
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect((input.element as HTMLInputElement).value).toBe('unlisted')
    await input.setValue('oth')
    expect(values()).toEqual(['other'])
    list()!.querySelector<HTMLElement>('[data-value="other"]')!.click()
    await wrapper.vm.$nextTick()
    expect((input.element as HTMLInputElement).value).toBe('other')
    expect(document.activeElement).toBe(input.element)
    expect(list()).toBeNull()
  })
  it('selects candidates with arrows and Enter without submitting the surrounding form', async () => {
    const input = render()
    const submit = vi.fn()
    const form = document.createElement('form')
    form.addEventListener('submit', submit)
    document.body.append(form)
    form.append(wrapper.element)
    await input.trigger('keydown', { key: 'ArrowUp' })
    expect(input.attributes('aria-activedescendant')).toBe('model-choice-option-2')
    await input.trigger('keydown', { key: 'ArrowDown' })
    expect(input.attributes('aria-activedescendant')).toBe('model-choice-option-0')
    const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
    input.element.dispatchEvent(event)
    await wrapper.vm.$nextTick()
    expect(event.defaultPrevented).toBe(true)
    expect(submit).not.toHaveBeenCalled()
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['Model-A'])
    expect(list()).toBeNull()
    form.remove()
  })
  it.each([
    ['ArrowDown', 'model-choice-option-0', 'Model-A'],
    ['ArrowUp', 'model-choice-option-2', 'other'],
  ])(
    'browses all candidates with %s from an unlisted model without clearing it',
    async (key, id, selected) => {
      const input = render('private-unlisted-model')
      await input.trigger('focus')
      expect(list()).toBeNull()
      await input.trigger('keydown', { key })
      expect(values()).toEqual(['Model-A', 'model-b', 'other'])
      expect(input.attributes('aria-activedescendant')).toBe(id)
      expect((input.element as HTMLInputElement).value).toBe('private-unlisted-model')
      expect(wrapper.emitted('update:modelValue')).toBeUndefined()
      await input.trigger('keydown', { key: 'Enter' })
      expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([selected])
      expect(list()).toBeNull()
    },
  )
  it('keeps the filter when navigating an open filtered menu and resumes filtering after keyboard expansion', async () => {
    const input = render('unlisted')
    await input.trigger('keydown', { key: 'ArrowDown' })
    expect(values()).toEqual(['Model-A', 'model-b', 'other'])
    await input.setValue('model')
    expect(values()).toEqual(['Model-A', 'model-b'])
    await input.trigger('keydown', { key: 'ArrowUp' })
    expect(values()).toEqual(['Model-A', 'model-b'])
    expect(input.attributes('aria-activedescendant')).toBe('model-choice-option-1')
    await input.trigger('keydown', { key: 'ArrowDown' })
    expect(values()).toEqual(['Model-A', 'model-b'])
    expect(input.attributes('aria-activedescendant')).toBe('model-choice-option-0')
    expect((input.element as HTMLInputElement).value).toBe('model')
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['model'])
  })
  it('preserves text Home/End, Space and composition Enter rather than selecting a candidate', async () => {
    const input = render()
    await input.trigger('keydown', { key: 'ArrowDown' })
    for (const key of ['Home', 'End', ' ']) {
      const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
      input.element.dispatchEvent(event)
      expect(event.defaultPrevented).toBe(false)
    }
    const composition = new KeyboardEvent('keydown', {
      key: 'Enter',
      isComposing: true,
      bubbles: true,
      cancelable: true,
    })
    input.element.dispatchEvent(composition)
    await wrapper.vm.$nextTick()
    expect(composition.defaultPrevented).toBe(false)
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    expect(list()).not.toBeNull()
  })
  it('closes on Escape, Tab, blur and outside interaction without clearing the input', async () => {
    const input = render('model', ['model-a', 'model-b'])
    for (const key of ['Escape', 'Tab']) {
      await input.trigger('click')
      const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
      input.element.dispatchEvent(event)
      await wrapper.vm.$nextTick()
      expect(list()).toBeNull()
      expect(event.defaultPrevented).toBe(key === 'Escape')
    }
    await input.trigger('click')
    await input.trigger('blur')
    expect(list()).toBeNull()
    await input.trigger('click')
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }))
    await wrapper.vm.$nextTick()
    expect(list()).toBeNull()
    expect((input.element as HTMLInputElement).value).toBe('model')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  })
  it('closes when disabled and rejects reopening or selection through a disabled fieldset', async () => {
    const input = render()
    await input.trigger('click')
    await wrapper.setProps({ disabled: true })
    expect(list()).toBeNull()
    expect(input.attributes('disabled')).toBeDefined()
    await input.trigger('keydown', { key: 'ArrowDown' })
    expect(list()).toBeNull()
    await wrapper.setProps({ disabled: false })
    const fieldset = document.createElement('fieldset')
    document.body.append(fieldset)
    fieldset.append(wrapper.element)
    await input.trigger('click')
    fieldset.disabled = true
    list()!.querySelector<HTMLElement>('[role="option"]')!.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await input.trigger('blur')
    await wrapper.get('.app-combobox-toggle').trigger('click')
    expect(list()).toBeNull()
    fieldset.remove()
  })
  it('keeps the active value stable on list refresh and refuses a removed candidate', async () => {
    const input = render()
    await input.trigger('keydown', { key: 'ArrowDown' })
    await input.trigger('keydown', { key: 'ArrowDown' })
    await wrapper.setProps({ options: ['model-b', 'other', 'other'] })
    expect(values()).toEqual(['model-b', 'other'])
    expect(input.attributes('aria-activedescendant')).toBe('model-choice-option-0')
    await wrapper.setProps({ options: ['new'] })
    expect(input.attributes('aria-activedescendant')).toBeUndefined()
    await input.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await wrapper.setProps({ options: [] })
    await input.trigger('click')
    expect(list()).toBeNull()
  })
  it('tracks viewport space and removes teleported content and listeners on unmount', async () => {
    const input = render()
    const root = wrapper.get('.app-combobox')
    const bounds = vi.spyOn(root.element, 'getBoundingClientRect').mockReturnValue({
      left: 100,
      top: window.innerHeight - 65,
      bottom: window.innerHeight - 20,
      width: 240,
    } as DOMRect)
    await input.trigger('click')
    expect(list()?.style.bottom).toBe('71px')
    expect(list()?.style.top).toBe('')
    bounds.mockReturnValue({ left: 40, top: 20, bottom: 62, width: 220 } as DOMRect)
    window.dispatchEvent(new Event('scroll'))
    await wrapper.vm.$nextTick()
    expect(list()?.style.top).toBe('68px')
    expect(list()?.style.bottom).toBe('')
    window.dispatchEvent(new Event('resize'))
    await wrapper.vm.$nextTick()
    expect(list()?.style.width).toBe('220px')
    const documentRemove = vi.spyOn(document, 'removeEventListener')
    const windowRemove = vi.spyOn(window, 'removeEventListener')
    wrapper.unmount()
    expect(list()).toBeNull()
    expect(documentRemove).toHaveBeenCalledWith('pointerdown', expect.any(Function))
    expect(windowRemove).toHaveBeenCalledWith('resize', expect.any(Function))
    expect(windowRemove).toHaveBeenCalledWith('scroll', expect.any(Function), true)
    window.dispatchEvent(new Event('resize'))
    expect(list()).toBeNull()
  })
})
