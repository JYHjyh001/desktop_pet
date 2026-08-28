import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PetChatWindow from './PetChatWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

function installPetChatCommands(theme: 'light' | 'animal-island') {
  tauriCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_public_config') {
      return {
        drawer: {
          theme,
          chatTypewriterEnabled: true,
          chatNarrationEnabled: false,
          chatMusicLinkEnabled: true,
        },
        ai: { enabled: false, provider: 'custom', model: 'test-model' },
      }
    }
    if (command === 'get_current_pet_skin') return null
    if (command === 'get_companion_messages') return []
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
