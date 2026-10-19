export type ImageReplyPart =
  | { type: 'text'; text: string }
  | { type: 'image'; text: string; src: string }

const maxImageSourceLength = 4 * 1024 * 1024

export function safeChatImageSource(source: string): string {
  if (!source || source.length > maxImageSourceLength) return ''
  if (/^pet-attachment:[a-f0-9]{32}$/.test(source)) return source
  if (/^data:image\/(?:png|jpeg|webp|gif);base64,[A-Za-z0-9+/]+={0,2}$/.test(source)) return source
  try {
    const url = new URL(source)
    return url.protocol === 'https:' && !url.username && !url.password ? url.href : ''
  } catch {
    return ''
  }
}

/** Extract images before narration parsing; keep URL parentheses out of action text. */
export function splitChatImages(content: string): ImageReplyPart[] {
  const parts: ImageReplyPart[] = []
  let cursor = 0
  let search = 0
  let count = 0
  while (search < content.length) {
    const start = content.indexOf('![', search)
    if (start < 0) break
    const labelEnd = content.indexOf('](', start + 2)
    if (labelEnd < 0) break
    const targetStart = labelEnd + 2
    let depth = 1
    let end = targetStart
    for (; end < content.length; end += 1) {
      if (content[end] === '(') depth += 1
      if (content[end] === ')') depth -= 1
      if (depth === 0) break
    }
    if (depth !== 0) { search = targetStart; continue }
    if (start > cursor) parts.push({ type: 'text', text: content.slice(cursor, start) })
    const target = content.slice(targetStart, end).trim()
    const source = target.startsWith('<')
      ? target.slice(1, target.indexOf('>') < 0 ? 0 : target.indexOf('>'))
      : target.replace(/\s+["'][\s\S]*["']$/, '')
    count += 1
    parts.push({
      type: 'image',
      text: content.slice(start + 2, labelEnd).trim().slice(0, 200) || 'AI 返回的图片',
      src: count <= 8 ? safeChatImageSource(source) : '',
    })
    cursor = end + 1
    search = cursor
  }
  if (cursor < content.length) parts.push({ type: 'text', text: content.slice(cursor) })
  return parts
}

export function chatTextContext(content: string): string {
  return splitChatImages(content).map((part) => part.type === 'image' ? '[图片]' : part.text).join('')
}
