import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import StoryWindow from './StoryWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

function installStoryCommands(theme: 'light' | 'animal-island') {
  tauriCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_public_config') {
      return {
        drawer: { theme },
        ai: { enabled: false, provider: 'custom', model: 'test-model' },
      }
    }
    if (command === 'list_companions' || command === 'list_story_saves') return []

    throw new Error(`Unexpected Tauri command in story smoke test: ${command}`)
  })
}

describe('StoryWindow accessibility', () => {
  it.each(['light', 'animal-island'] as const)(
    'supports the story radio keyboard pattern in the %s theme',
    async (theme) => {
      installStoryCommands(theme)
      const wrapper = mount(StoryWindow, { attachTo: document.body })
      await flushPromises()

      expect(wrapper.get('.story-window').classes()).toContain(`theme-${theme}`)
      expect(wrapper.get('.story-notice').attributes('role')).toBe('status')

      const radios = wrapper.findAll<HTMLButtonElement>('[role="radio"]')
      expect(radios).toHaveLength(2)
      expect(radios[0].attributes('aria-checked')).toBe('true')
      expect(radios[0].attributes('tabindex')).toBe('0')
      expect(radios[1].attributes('tabindex')).toBe('-1')

      radios[0].element.focus()
      await radios[0].trigger('keydown', { key: 'ArrowRight' })

      expect(radios[0].attributes('aria-checked')).toBe('false')
      expect(radios[1].attributes('aria-checked')).toBe('true')
      expect(document.activeElement).toBe(radios[1].element)

      wrapper.unmount()
    },
  )
})
