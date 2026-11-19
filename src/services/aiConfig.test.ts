import { describe, expect, it } from 'vitest'
import {
  aiConfigStatus,
  channelNameError,
  DEFAULT_AI_SYSTEM_PROMPT,
  emptyAiConfig,
  OFFICIAL_AI_CHANNEL_ID,
  OFFICIAL_AI_CHANNEL_NAME,
  endpointForProtocol,
  OPENAI_ENDPOINT,
} from './aiConfig'
import type { AiChannel, AiConfig } from '../types/desktop'

const config: AiConfig = {
  protocol: 'openai',
  endpoint: OPENAI_ENDPOINT,
  model: 'fixture-model',
  apiKey: 'fictional',
  systemPrompt: '',
}

describe('AI configuration guidance', () => {
  it('does not fill missing fields with defaults', () => {
    expect(aiConfigStatus()).toEqual({
      configured: false,
      missing: ['protocol', 'endpoint', 'model'],
    })
    expect(aiConfigStatus({ ...config, model: '' }).missing).toEqual(['model'])
  })
  it('requires a key for the official OpenAI endpoint', () => {
    expect(aiConfigStatus({ ...config, apiKey: '' }).missing).toEqual(['apiKey'])
    expect(aiConfigStatus(config).configured).toBe(true)
    expect(
      aiConfigStatus({
        ...config,
        protocol: 'openai-responses',
        endpoint: OPENAI_ENDPOINT,
      }).configured,
    ).toBe(true)
    expect(aiConfigStatus({ ...config, protocol: 'openai-responses', apiKey: '' }).missing).toEqual(
      ['apiKey'],
    )
    expect(aiConfigStatus({ ...config, endpoint: 'api.openai.com', apiKey: '' }).missing).toEqual([
      'apiKey',
    ])
  })
  it('supports explicitly configured no-key custom and Ollama endpoints', () => {
    expect(
      aiConfigStatus({ ...config, endpoint: 'http://localhost:8000/chat', apiKey: '' }).configured,
    ).toBe(true)
    expect(
      aiConfigStatus({ ...config, endpoint: 'http://localhost:11434', apiKey: '' }).configured,
    ).toBe(true)
  })
  it('marks each absent required field as unconfigured', () => {
    for (const field of ['protocol', 'endpoint', 'model'] as const) {
      const incomplete: Partial<AiConfig> = { ...config }
      delete incomplete[field]
      expect(aiConfigStatus(incomplete).missing).toEqual([field])
    }
  })
  it('rejects unsupported protocols and unsafe or invalid endpoint syntax', () => {
    expect(
      aiConfigStatus({ ...config, protocol: 'other' as AiConfig['protocol'] }).missing,
    ).toEqual(['protocol'])
    for (const endpoint of [
      '',
      'not-url',
      'not-url:8443',
      'http:example.com/chat',
      'http:/example.com',
      'https:example.com',
      'https:',
      'https:/example.com/chat',
      'https:///example.com',
      '/example.com',
      '//example.com',
      'https://example.com/has space',
      'https://example.com/chat\n/completions',
      'https://example.com/chat\u0000',
      'https://example.com/chat\u0085path',
      'file:///test',
      'ftp://example.com',
      'example.com:bad-port',
      'example.com:65536',
      'example.com\\v1',
      'https://example.com\\v1',
      'https://user:pass@example.com/chat',
      'user:pass@example.com',
      'user@example.com',
      'example.com#',
      'example.com#fragment',
      'example.com/has space',
      'example.com/v1\n/models',
      'example.com/v1\u0000',
      'example.com/v1\u0085',
      '::1',
      '[invalid-ipv6]',
      'https://example.com/chat#fragment',
      'https://example.com/chat#',
    ]) {
      expect(aiConfigStatus({ ...config, endpoint }).missing).toEqual(['endpoint'])
    }
  })
  it('allows surrounding whitespace and explicit uppercase HTTP schemes', () => {
    expect(
      aiConfigStatus({ ...config, endpoint: '  HTTP://localhost:8000/chat  ' }).configured,
    ).toBe(true)
  })
  it('accepts domains, version bases, IP hosts and local services without requiring a scheme', () => {
    for (const endpoint of [
      'relay.example.com',
      'relay.example.com:8443',
      'relay.example.com/v1',
      'relay.example.com/proxy/v1/?tenant=test',
      '  relay.example.com  ',
      'localhost',
      'localhost:11434',
      '127.0.0.1:11434',
      '[::1]:11434',
      '[2001:db8::1]/v1',
      'http://localhost:11434',
      'http://[::1]:11434',
      'http://intranet:8000',
      'https://relay.example.com/v1?tenant=test',
    ]) {
      expect(aiConfigStatus({ ...config, endpoint, apiKey: '' }).configured, endpoint).toBe(true)
    }
  })
})

describe('named AI channel helpers', () => {
  const channels: AiChannel[] = [
    { id: OFFICIAL_AI_CHANNEL_ID, name: OFFICIAL_AI_CHANNEL_NAME, config: emptyAiConfig() },
    { id: 'first', name: 'My Relay', config: emptyAiConfig() },
    { id: 'second', name: '备用渠道', config: emptyAiConfig() },
  ]

  it('creates independent empty configurations using the backend default prompt', () => {
    const first = emptyAiConfig()
    const second = emptyAiConfig()
    expect(first).toEqual({
      protocol: 'openai',
      endpoint: '',
      apiKey: '',
      model: '',
      systemPrompt: DEFAULT_AI_SYSTEM_PROMPT,
    })
    expect(first).not.toBe(second)
    first.apiKey = 'fictional'
    first.endpoint = 'relay.example.com'
    first.model = 'fixture-model'
    expect(second).toEqual(emptyAiConfig())
    expect(DEFAULT_AI_SYSTEM_PROMPT).toBe(
      'You are a professional translator. Translate the following text from {source_lang} to {target_lang}. Only output the translation result, no explanations.',
    )
  })

  it('rejects empty names and control characters before trimming', () => {
    for (const name of ['', '   ', '\u3000']) {
      expect(channelNameError(name, channels)).toBe('channelNameRequired')
    }
    for (const name of ['\nValid', 'Valid\r', '\tValid ', 'A\u0000B', 'A\u0085B', '\u007f']) {
      expect(channelNameError(name, channels), name).toBe('channelNameInvalid')
    }
  })

  it('counts Unicode characters after trimming rather than UTF-16 code units', () => {
    expect(channelNameError(`  ${'𠮷'.repeat(60)}  `, channels)).toBe('')
    expect(channelNameError('𠮷'.repeat(61), channels)).toBe('channelNameTooLong')
    expect(channelNameError('译'.repeat(60), channels)).toBe('')
    expect(channelNameError('译'.repeat(61), channels)).toBe('channelNameTooLong')
  })

  it('compares trimmed names case-insensitively and excludes only the renamed identity', () => {
    expect(channelNameError(' my RELAY ', channels)).toBe('channelNameDuplicate')
    expect(channelNameError('my relay', channels, 'first')).toBe('')
    expect(channelNameError('备用渠道', channels, 'first')).toBe('channelNameDuplicate')
    expect(channelNameError(' MY RELAY ', channels, 'second')).toBe('channelNameDuplicate')
    expect(channelNameError('新渠道', channels)).toBe('')
    expect(channelNameError('  New Relay  ', channels)).toBe('')
  })

  it('reserves both official display names even without an official entry or when excluded', () => {
    for (const name of [
      OFFICIAL_AI_CHANNEL_NAME,
      ' up翻译官方 ai ',
      'UPfanyi official AI',
      ' UPFANYI OFFICIAL ai ',
    ]) {
      expect(channelNameError(name, [], OFFICIAL_AI_CHANNEL_ID)).toBe('channelNameDuplicate')
    }
  })
})

describe('explicit API switching', () => {
  it('preserves the proxy prefix and query when switching OpenAI APIs', () => {
    const proxy = {
      ...config,
      endpoint: 'https://proxy.example/prefix/v1/chat/completions/?tenant=test',
    }
    const endpoint = endpointForProtocol(proxy, 'openai-responses')
    expect(endpoint).toBe('https://proxy.example/prefix/v1/responses?tenant=test')
    expect(
      endpointForProtocol({ ...proxy, protocol: 'openai-responses', endpoint }, 'openai'),
    ).toBe('https://proxy.example/prefix/v1/chat/completions?tenant=test')
  })
  it('preserves custom paths and invalid drafts on API switches', () => {
    for (const endpoint of ['https://proxy.example/custom', 'not a complete URL']) {
      expect(endpointForProtocol({ ...config, endpoint }, 'openai-responses')).toBe(endpoint)
    }
  })
  it('fills an empty service address only on an explicit interface switch', () => {
    for (const protocol of ['openai', 'openai-responses'] as const) {
      expect(endpointForProtocol({ ...config, endpoint: '' }, protocol)).toBe(OPENAI_ENDPOINT)
    }
  })
  it('preserves base-address text including its optional scheme, prefix, version and query on API switches', () => {
    for (const endpoint of [
      'relay.example.com',
      'relay.example.com:8443/v1?tenant=test',
      'https://relay.example.com/proxy/v1/',
      'https://api.openai.com/',
    ]) {
      expect(endpointForProtocol({ ...config, endpoint }, 'openai-responses')).toBe(endpoint)
      expect(
        endpointForProtocol({ ...config, protocol: 'openai-responses', endpoint }, 'openai'),
      ).toBe(endpoint)
    }
  })
})
