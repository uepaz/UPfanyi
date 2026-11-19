import type { AiChannel, AiConfig } from '../types/desktop'

export const OPENAI_ENDPOINT = 'https://api.openai.com'
export const UPFANYI_AI_ENDPOINT = 'https://uptoken.top'
export const OFFICIAL_AI_CHANNEL_ID = 'upfanyi'
export const OFFICIAL_AI_CHANNEL_NAME = 'UP翻译官方 AI'
export const DEFAULT_AI_SYSTEM_PROMPT =
  'You are a professional translator. Translate the following text from {source_lang} to {target_lang}. Only output the translation result, no explanations.'

/** 新渠道的空白草稿独立创建，避免引用复用导致凭据或编辑内容串到其它渠道。 */
export function emptyAiConfig(): AiConfig {
  return {
    protocol: 'openai',
    endpoint: '',
    apiKey: '',
    model: '',
    systemPrompt: DEFAULT_AI_SYSTEM_PROMPT,
  }
}

/** 与 Rust 保存边界一致：控制字符先于 trim 检查，不能用首尾换行绕过校验。 */
export function channelNameError(name: string, channels: AiChannel[], exceptId?: string) {
  if (/\p{Cc}/u.test(name)) return 'channelNameInvalid'
  const normalized = name.trim()
  if (!normalized) return 'channelNameRequired'
  if ([...normalized].length > 60) return 'channelNameTooLong'
  const folded = normalized.toLowerCase()
  if (
    [OFFICIAL_AI_CHANNEL_NAME.toLowerCase(), 'upfanyi official ai'].includes(folded) ||
    channels.some(
      (channel) => channel.id !== exceptId && channel.name.trim().toLowerCase() === folded,
    )
  )
    return 'channelNameDuplicate'
  return ''
}

/** 仅解析用于表单提示，不修改草稿；实际协议路径统一由 Rust 生成。 */
function serviceAddress(value: string) {
  const text = value.trim()
  if (!text || /[\p{White_Space}\p{Cc}\\#]/u.test(text)) return undefined
  const explicitHttp = /^https?:\/\//i.test(text)
  // 不把缺斜线的 HTTP 协议、其它协议或相对路径当作域名修补。
  if (!explicitHttp && (/^https?:/i.test(text) || text.includes('://') || text.startsWith('/')))
    return undefined
  if (explicitHttp && /^https?:\/\/\//i.test(text)) return undefined
  try {
    const url = new URL(explicitHttp ? text : `https://${text}`)
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password)
      return undefined
    if (
      !explicitHttp &&
      url.hostname !== 'localhost' &&
      !url.hostname.includes('.') &&
      !url.hostname.startsWith('[')
    )
      return undefined
    return url
  } catch {
    return undefined
  }
}

/** 切换接口只替换已知后缀，基础地址或自定义路径保持原样。 */
export function endpointForProtocol(config: AiConfig, protocol: AiConfig['protocol']) {
  const endpoint = config.endpoint.trim()
  if (!endpoint) return OPENAI_ENDPOINT
  const url = serviceAddress(endpoint)
  if (url && /\/(chat\/completions|responses)\/?$/.test(url.pathname)) {
    url.pathname = url.pathname.replace(
      /\/(chat\/completions|responses)\/?$/,
      protocol === 'openai' ? '/chat/completions' : '/responses',
    )
    return url.toString()
  }
  return config.endpoint
}

/** 仅提示字段完整性；连接、认证、额度和正式请求校验仍由 Rust 判断。 */
export function aiConfigStatus(config?: Partial<AiConfig>) {
  const missing: (keyof AiConfig)[] = []
  if (config?.protocol !== 'openai' && config?.protocol !== 'openai-responses')
    missing.push('protocol')
  if (!config?.endpoint?.trim()) missing.push('endpoint')
  if (!config?.model?.trim()) missing.push('model')
  // 只读取已保存或正在编辑的字段，缺失服务地址不得暗中换成默认服务。
  const endpoint = serviceAddress(config?.endpoint ?? '')
  if (!endpoint && !missing.includes('endpoint')) missing.push('endpoint')
  if (
    (config?.protocol === 'openai' || config?.protocol === 'openai-responses') &&
    endpoint?.hostname === 'api.openai.com' &&
    !config.apiKey?.trim()
  )
    missing.push('apiKey')
  return { configured: missing.length === 0, missing }
}
