import { defineComponent, ref } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ConfirmAction from './ConfirmAction.vue'
const targets: HTMLElement[] = []
afterEach(() => targets.splice(0).forEach((target) => target.remove()))
describe('keyboard confirmation', () => {
  it('starts on cancel, traps Tab, closes on Escape and restores focus', async () => {
    const target = document.createElement('div')
    document.body.appendChild(target)
    targets.push(target)
    const Parent = defineComponent({
      components: { ConfirmAction },
      setup: () => ({ open: ref(false) }),
      template:
        '<button id="open-confirm" @click="open = true">Open</button><ConfirmAction v-if="open" message="Delete?" :busy="false" @cancel="open = false" />',
    })
    const wrapper = mount(Parent, { attachTo: target })
    const opener = wrapper.get('#open-confirm').element as HTMLButtonElement
    opener.focus()
    await wrapper.get('#open-confirm').trigger('click')
    const buttons = wrapper.findAll('.confirmation button')
    expect(document.activeElement).toBe(buttons[0]!.element)
    await buttons[0]!.trigger('keydown', { key: 'Tab', shiftKey: true })
    expect(document.activeElement).toBe(buttons[1]!.element)
    await buttons[1]!.trigger('keydown', { key: 'Tab' })
    expect(document.activeElement).toBe(buttons[0]!.element)
    await buttons[0]!.trigger('keydown', { key: 'Escape' })
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
    expect(document.activeElement).toBe(opener)
    wrapper.unmount()
  })
  it('keeps an in-progress destructive action open when Escape is pressed', async () => {
    const target = document.createElement('div')
    document.body.appendChild(target)
    targets.push(target)
    const wrapper = mount(ConfirmAction, {
      props: { message: 'Delete?', busy: true },
      attachTo: target,
    })
    await wrapper.get('[role="alertdialog"]').trigger('keydown', { key: 'Escape' })
    expect(wrapper.emitted('cancel')).toBeUndefined()
    wrapper.unmount()
  })
})
