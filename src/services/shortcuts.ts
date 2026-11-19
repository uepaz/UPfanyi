type Modifiers = Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>

export function shortcutModifiers(event: Modifiers): string[] {
  return [
    event.ctrlKey ? 'Ctrl' : '',
    event.altKey ? 'Alt' : '',
    event.shiftKey ? 'Shift' : '',
    event.metaKey ? 'Super' : '',
  ].filter(Boolean)
}

const namedKeys = new Set([
  'Backquote',
  'Backslash',
  'BracketLeft',
  'BracketRight',
  'Comma',
  'Equal',
  'Minus',
  'Period',
  'Quote',
  'Semicolon',
  'Slash',
  'Backspace',
  'CapsLock',
  'Enter',
  'Space',
  'Tab',
  'Escape',
  'Delete',
  'End',
  'Home',
  'Insert',
  'PageDown',
  'PageUp',
  'PrintScreen',
  'ScrollLock',
  'Pause',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'NumLock',
  'NumpadAdd',
  'NumpadDecimal',
  'NumpadDivide',
  'NumpadEnter',
  'NumpadEqual',
  'NumpadMultiply',
  'NumpadSubtract',
  'AudioVolumeDown',
  'AudioVolumeUp',
  'AudioVolumeMute',
  'MediaPlay',
  'MediaPause',
  'MediaPlayPause',
  'MediaStop',
  'MediaTrackNext',
  'MediaTrackPrevious',
])

/** 与锁定的原生快捷键解析器一致；使用 code 保留位置，避免 Shift 字符和加号分隔冲突。 */
export function shortcutFromEvent(event: Modifiers & Pick<KeyboardEvent, 'code'>) {
  let key: string
  if (/^Key[A-Z]$/.test(event.code)) key = event.code.slice(3)
  else if (/^Digit[0-9]$/.test(event.code)) key = event.code.slice(5)
  else if (/^Numpad[0-9]$/.test(event.code) || /^F([1-9]|1[0-9]|2[0-4])$/.test(event.code))
    key = event.code
  else if (namedKeys.has(event.code)) key = event.code
  else return undefined
  return [...shortcutModifiers(event), key].join('+')
}
