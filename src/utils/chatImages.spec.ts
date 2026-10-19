import { describe, expect, it } from 'vitest'
import { chatTextContext, safeChatImageSource, splitChatImages } from './chatImages'

describe('chat image replies', () => {
  it('keeps mixed image order, URL parentheses and titles intact', () => {
    const parts = splitChatImages('前文 ![小猫](https://example.test/cat(1).png "猫") 后文 ![狗](<https://example.test/dog.png>)')
    expect(parts.map((part) => part.type)).toEqual(['text', 'image', 'text', 'image'])
    expect(parts[1]).toEqual({ type: 'image', text: '小猫', src: 'https://example.test/cat(1).png' })
  })
  it('supports image-only replies and removes payloads from next-turn context', () => {
    const text = '![图](data:image/png;base64,AAAA)'
    expect(splitChatImages(text)).toEqual([{ type: 'image', text: '图', src: 'data:image/png;base64,AAAA' }])
    expect(chatTextContext(text)).toBe('[图片]')
    expect(chatTextContext('前' + text + '后')).toBe('前[图片]后')
  })
  it('blocks executable, local and oversized sources without rendering HTML', () => {
    for (const source of ['javascript:alert(1)', 'file:///secret.png', 'http://example.test/image.png', 'data:image/svg+xml;base64,AAAA', 'https://fixture@example.test/image.png']) {
      expect(safeChatImageSource(source)).toBe('')
    }
    expect(safeChatImageSource(`data:image/png;base64,${'A'.repeat(4 * 1024 * 1024)}`)).toBe('')
    expect(splitChatImages('<img src=x onerror=alert(1)>')[0].type).toBe('text')
  })
})
