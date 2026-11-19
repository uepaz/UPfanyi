import { describe, expect, it } from 'vitest'
import { shortcutFromEvent } from './shortcuts'

function event(code: string, flags: Partial<KeyboardEvent> = {}) {
  return { code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...flags }
}

describe('native shortcut format', () => {
  it('keeps physical digits and punctuation when their typed character would differ', () => {
    expect(shortcutFromEvent(event('Digit1', { ctrlKey: true, shiftKey: true, key: '!' }))).toBe(
      'Ctrl+Shift+1',
    )
    expect(shortcutFromEvent(event('Equal', { ctrlKey: true, shiftKey: true, key: '+' }))).toBe(
      'Ctrl+Shift+Equal',
    )
    expect(shortcutFromEvent(event('KeyA', { key: 'ф', altKey: true }))).toBe('Alt+A')
  })
  it('orders all modifiers before the key and keeps numpad and navigation identities', () => {
    expect(
      shortcutFromEvent(
        event('F24', { ctrlKey: true, altKey: true, shiftKey: true, metaKey: true }),
      ),
    ).toBe('Ctrl+Alt+Shift+Super+F24')
    expect(shortcutFromEvent(event('Numpad1'))).toBe('Numpad1')
    expect(shortcutFromEvent(event('Digit1'))).toBe('1')
    expect(shortcutFromEvent(event('ArrowUp', { altKey: true }))).toBe('Alt+ArrowUp')
    expect(shortcutFromEvent(event('MediaTrackPrevious'))).toBe('MediaTrackPrevious')
  })
  it('rejects unsupported or modifier-only codes instead of inventing a shortcut', () => {
    for (const code of [
      'ControlLeft',
      'AltRight',
      'ShiftLeft',
      'MetaRight',
      'F25',
      'Unidentified',
      'IntlYen',
      '',
    ])
      expect(shortcutFromEvent(event(code))).toBeUndefined()
  })
})
