import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PetChatWindow from './PetChatWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

function installPetChatCommands(theme: 'light' | 'animal-island', history: unknown[] = [], enabled = false) {
  tauriCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_public_config') {
      return {
        drawer: {
          theme,
          chatTypewriterEnabled: true,
          chatNarrationEnabled: false,
          chatMusicLinkEnabled: false,
        },
        ai: { enabled, provider: 'custom', model: 'test-model' },
      }
    }
    if (command === 'get_current_pet_skin') return null
    if (command === 'get_companion_messages') return history
    if (command === 'get_companion_message_page') return { messages: history, nextBefore: null, hasMore: false, companionId: 'test-companion' }
    if (command === 'load_chat_image') return 'data:image/png;base64,AAAA'
    if (command === 'send_pet_chat_message') {
      const message = '新图片 ![图](data:image/png;base64,BBBB)'
      history.push({ id: 10, role: 'assistant', content: message, createdAt: '10' })
      return { message, provider: 'custom', model: 'test-model' }
    }
    if (command === 'get_current_companion') {
      return { id: 'test-companion', name: '测试伴侣', firstMessage: '测试问候' }
    }
    if (command === 'get_current_companion_status') {
      return {
        favorabilityEnabled: true,
        favorability: 0,
        relationshipStage: 'stranger',
        relationshipStageName: '初识',
        mood: '平静',
        trust: 0,
        intimacy: 0,
        lastChangeReason: '',
        lastInteractionTime: null,
      }
    }

    throw new Error(`Unexpected Tauri command in pet chat smoke test: ${command}`)
  })
}

describe('PetChatWindow accessibility', () => {
  it.each(['light', 'animal-island'] as const)('replaces history pages and reads a deferred record alone in %s', async (theme) => {
    installPetChatCommands(theme)
    const fallback = tauriCommandMock.getMockImplementation()!
    tauriCommandMock.mockImplementation(async (command: string, payload) => {
      if (command === 'get_companion_message_page') {
        return payload?.before
          ? { messages: [{ id: 1, role: 'assistant', content: '较大消息', contentDeferred: true, createdAt: '1' }], nextBefore: 1, hasMore: false, companionId: 'test-companion' }
          : { messages: Array.from({ length: 20 }, (_, index) => ({ id: index + 21, role: 'assistant', content: `recent-${index}`, createdAt: '1' })), nextBefore: 21, hasMore: true, companionId: 'test-companion' }
      }
      if (command === 'get_companion_message') return { id: 1, role: 'assistant', content: '完整旧消息', createdAt: '1' }
      return fallback(command, payload)
    })
    const wrapper = mount(PetChatWindow)
    await flushPromises()
    expect(wrapper.findAll('.pet-chat-message')).toHaveLength(20)
    await wrapper.get('.pet-chat-history-controls button').trigger('click')
    await flushPromises()
    expect(tauriCommandMock).toHaveBeenCalledWith('get_companion_message_page', { before: 21, limit: 20 })
    expect(wrapper.findAll('.pet-chat-message')).toHaveLength(1)
    const deferred = wrapper.findAll('button').find((button) => button.text() === '读取完整消息')!
    await deferred.trigger('click')
    await flushPromises()
    expect(wrapper.findAll('.pet-chat-message')).toHaveLength(1)
    expect(wrapper.text()).toContain('完整旧消息')
    await wrapper.get('.pet-chat-history-controls button').trigger('click')
    await flushPromises()
    expect(wrapper.findAll('.pet-chat-message')).toHaveLength(20)
    wrapper.unmount()
  })
  it('sends image markers with narration disabled and displays the new image immediately', async () => {
    installPetChatCommands('light', [
      { id: 1, role: 'assistant', content: '![图片](data:image/png;base64,AAAA)', createdAt: '1' },
    ], true)
    const wrapper = mount(PetChatWindow, { attachTo: document.body })
    await flushPromises()
    await wrapper.get('textarea').setValue('继续')
    await wrapper.get('form.pet-chat-input').trigger('submit')
    await flushPromises()
    const call = tauriCommandMock.mock.calls.find(([command]) => command === 'send_pet_chat_message')
    expect(call?.[1].messages[0].content).toBe('[图片]')
    expect(JSON.stringify(call?.[1])).not.toContain('base64')
    expect(wrapper.findAll('.chat-reply-image')).toHaveLength(2)
    expect(wrapper.find('.typing').exists()).toBe(false)
    wrapper.unmount()
  })
  it.each(['light', 'animal-island'] as const)(
    'restores mixed and image-only history with narration disabled in %s', async (theme) => {
      installPetChatCommands(theme, [
        { id: 1, role: 'assistant', content: '给你看 ![小猫](https://example.test/cat(1).png) （微笑）', createdAt: '1' },
        { id: 2, role: 'assistant', content: '![图片](data:image/png;base64,AAAA)', createdAt: '2' },
      ])
      const wrapper = mount(PetChatWindow, { attachTo: document.body })
      await flushPromises()
      expect(wrapper.get('.pet-chat-window').classes()).toContain(`theme-${theme}`)
      expect(wrapper.findAll('.chat-reply-image')).toHaveLength(2)
      expect(wrapper.get('.chat-reply-image img').attributes('src')).toBe('data:image/png;base64,AAAA')
      expect(tauriCommandMock).toHaveBeenCalledWith('load_chat_image', { source: 'https://example.test/cat(1).png' })
      expect(wrapper.text()).toContain('给你看')
      expect(wrapper.findAll('.pet-chat-message-narration')).toHaveLength(0)
      wrapper.unmount()
    },
  )
  it.each(['light', 'animal-island'] as const)(
    'focuses chat settings and restores focus in the %s theme',
    async (theme) => {
      installPetChatCommands(theme)
      const wrapper = mount(PetChatWindow, { attachTo: document.body })
      await flushPromises()

      expect(wrapper.get('.pet-chat-window').classes()).toContain(`theme-${theme}`)
      expect(wrapper.get('.pet-chat-notice').attributes('role')).toBe('status')
      const settingsTrigger = wrapper.get<HTMLButtonElement>('button[title="对话设置"]')
      settingsTrigger.element.focus()
      await settingsTrigger.trigger('click')
      await flushPromises()

      const dialog = wrapper.get('[role="dialog"][aria-labelledby="pet-chat-settings-title"]')
      expect(dialog.attributes('aria-modal')).toBe('true')
      const closeButton = wrapper.get<HTMLButtonElement>('button[aria-label="关闭对话设置"]')
      expect(document.activeElement).toBe(closeButton.element)

      await dialog.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(
        wrapper.find('[role="dialog"][aria-labelledby="pet-chat-settings-title"]').exists(),
      ).toBe(false)
      expect(document.activeElement).toBe(settingsTrigger.element)

      wrapper.unmount()
    },
  )
})
