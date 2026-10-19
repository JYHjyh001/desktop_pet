import { mount, flushPromises } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { tauriCommandMock } from '../tests/tauriBridgeMock'
import ChatReplyImage from './ChatReplyImage.vue'

describe('ChatReplyImage', () => {
  it('expands, handles load failure and retries without credentials', async () => {
    tauriCommandMock.mockResolvedValue('data:image/png;base64,AAAA')
    const wrapper = mount(ChatReplyImage, { props: { src: 'https://example.test/image.png', alt: '图片说明' } })
    await flushPromises()
    expect(wrapper.get('img').attributes('src')).toMatch(/^data:/)
    expect(tauriCommandMock).toHaveBeenCalledWith('load_chat_image', { source: 'https://example.test/image.png' })
    expect(wrapper.get('img').attributes('referrerpolicy')).toBe('no-referrer')
    await wrapper.get('img').trigger('load')
    await wrapper.get('button').trigger('click')
    expect(wrapper.classes()).toContain('expanded')
    expect(wrapper.get('button').attributes('aria-expanded')).toBe('true')
    await wrapper.get('img').trigger('error')
    expect(wrapper.get('[role="status"]').text()).toContain('图片加载失败')
    expect(wrapper.find('img').exists()).toBe(false)
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(wrapper.find('img').exists()).toBe(true)
    wrapper.unmount()
  })
  it('never loads a disallowed source', () => {
    const wrapper = mount(ChatReplyImage, { props: { src: 'file:///secret.png', alt: '图片' } })
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain('图片地址或格式不支持')
    wrapper.unmount()
  })
  it('discards a late image result when the page becomes hidden', async () => {
    let complete!: (value: string) => void
    tauriCommandMock.mockImplementation(() => new Promise<string>((resolve) => { complete = resolve }))
    const original = Object.getOwnPropertyDescriptor(document, 'visibilityState')
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' })
    const wrapper = mount(ChatReplyImage, { props: { src: 'https://example.test/image.png', alt: '图片' } })
    await flushPromises()
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' })
    document.dispatchEvent(new Event('visibilitychange'))
    complete('data:image/png;base64,AAAA')
    await flushPromises()
    expect(wrapper.find('img').exists()).toBe(false)
    tauriCommandMock.mockResolvedValue('data:image/png;base64,BBBB')
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' })
    document.dispatchEvent(new Event('visibilitychange'))
    await flushPromises()
    expect(wrapper.get('img').attributes('src')).toContain('BBBB')
    wrapper.unmount()
    if (original) Object.defineProperty(document, 'visibilityState', original)
    else Reflect.deleteProperty(document, 'visibilityState')
  })
})
