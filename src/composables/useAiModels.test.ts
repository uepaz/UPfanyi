import { effectScope, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useAiModels } from './useAiModels'
import { desktop } from '../services/desktop'
import type { AiConfig } from '../types/desktop'

vi.mock('../services/desktop', () => ({
  desktop: { listAiModels: vi.fn() },
  errorMessage: (reason: unknown) => (reason instanceof Error ? reason.message : String(reason)),
}))

const scopes: ReturnType<typeof effectScope>[] = []
function render() {
  const config = ref<AiConfig>({
    protocol: 'openai',
    endpoint: 'https://example.com/v1/chat/completions',
    apiKey: 'fictional',
    model: '',
    systemPrompt: '',
  })
  const scope = effectScope()
  scopes.push(scope)
  const models = scope.run(() => useAiModels(() => config.value))!
  return { config, scope, ...models }
}
function deferred() {
  let resolve!: (value: string[]) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<string[]>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
beforeEach(() => vi.resetAllMocks())
afterEach(() => scopes.splice(0).forEach((scope) => scope.stop()))

describe('model list request isolation', () => {
  it('waits for an explicit request, snapshots an empty model and ignores duplicate clicks', async () => {
    const result = deferred()
    vi.mocked(desktop.listAiModels).mockReturnValue(result.promise)
    const { config, state, load } = render()
    expect(desktop.listAiModels).not.toHaveBeenCalled()
    const request = load()
    await load()
    expect(desktop.listAiModels).toHaveBeenCalledTimes(1)
    expect(vi.mocked(desktop.listAiModels).mock.calls[0]![0]).not.toBe(config.value)
    config.value = { ...config.value, model: 'manual', systemPrompt: 'edited' }
    expect(vi.mocked(desktop.listAiModels).mock.calls[0]![0].model).toBe('')
    result.resolve(['first', 'second'])
    await request
    expect(state).toEqual({ pending: false, error: '', loaded: true, items: ['first', 'second'] })
    expect(config.value.model).toBe('manual')
    config.value = { ...config.value, model: 'unlisted', systemPrompt: 'another prompt' }
    expect(state.items).toEqual(['first', 'second'])
  })

  it.each(['endpoint', 'apiKey', 'protocol'] as const)(
    'invalidates old results on %s edits even when changed back',
    async (field) => {
      const result = deferred()
      vi.mocked(desktop.listAiModels).mockReturnValue(result.promise)
      const { config, state, load } = render()
      const original = { ...config.value }
      const request = load()
      config.value = {
        ...config.value,
        [field]: field === 'protocol' ? 'openai-responses' : 'changed',
      }
      config.value = original
      result.resolve(['outdated'])
      await request
      expect(state).toEqual({ pending: false, error: '', items: [], loaded: false })
    },
  )

  it('keeps the newer request pending after an older failure and supports retry after current failure', async () => {
    const old = deferred()
    const current = deferred()
    vi.mocked(desktop.listAiModels)
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(current.promise)
      .mockResolvedValueOnce([])
    const { config, state, load } = render()
    const oldRequest = load()
    config.value.apiKey = 'edited'
    const currentRequest = load()
    old.reject(new Error('old failure'))
    await oldRequest
    expect(state.pending).toBe(true)
    expect(state.error).toBe('')
    current.reject(new Error('current failure'))
    await currentRequest
    expect(state.pending).toBe(false)
    expect(state.error).toBe('current failure')
    await load()
    expect(state).toEqual({ pending: false, error: '', items: [], loaded: true })
  })

  it('ignores an older success after a newer failure', async () => {
    const old = deferred()
    vi.mocked(desktop.listAiModels)
      .mockReturnValueOnce(old.promise)
      .mockRejectedValueOnce(new Error('current failure'))
    const { config, state, load } = render()
    const oldRequest = load()
    config.value.endpoint = 'http://localhost:8000/v1/responses'
    await load()
    old.resolve(['outdated'])
    await oldRequest
    expect(state).toEqual({ pending: false, error: 'current failure', items: [], loaded: false })
  })

  it('resets an identical draft without accepting its pending result', async () => {
    const result = deferred()
    vi.mocked(desktop.listAiModels).mockReturnValue(result.promise)
    const { state, load, reset } = render()
    const request = load()
    reset()
    result.reject(new Error('cancelled failure'))
    await request
    expect(state).toEqual({ pending: false, error: '', items: [], loaded: false })
  })

  it('does not change state or start requests after disposal', async () => {
    const result = deferred()
    vi.mocked(desktop.listAiModels).mockReturnValue(result.promise)
    const { state, scope, load } = render()
    const request = load()
    scope.stop()
    const stopped = { ...state }
    result.resolve(['outdated'])
    await request
    await load()
    expect(state).toEqual(stopped)
    expect(desktop.listAiModels).toHaveBeenCalledTimes(1)
  })
})
