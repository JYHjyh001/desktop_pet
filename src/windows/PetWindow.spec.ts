import { flushPromises, mount } from '@vue/test-utils'
import { mockWindows } from '@tauri-apps/api/mocks'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PetWindow from './PetWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

const eventMocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  unlisteners: [] as Array<ReturnType<typeof vi.fn>>,
  listen: vi.fn(),
  emit: vi.fn(async () => undefined),
}))

const shortcutMocks = vi.hoisted(() => ({
  register: vi.fn(async () => undefined),
  unregister: vi.fn(async () => undefined),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: eventMocks.listen,
  emit: eventMocks.emit,
}))

vi.mock('@tauri-apps/plugin-global-shortcut', () => ({
  register: shortcutMocks.register,
  unregister: shortcutMocks.unregister,
}))

function installPetWindowCommands(theme: 'light' | 'animal-island') {
  tauriCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_public_config') {
      return {
        drawer: { theme },
        shortcut: {
          petSingleClick: 'smartCodexOrDrawer',
          petDoubleClick: 'toggleDrawer',
          petRightClick: 'petMenu',
          translateSelection: 'Ctrl+Alt+T',
          translateSelectionConsent: true,
        },
      }
    }
    if (command === 'get_current_pet_skin') return null
    if (command === 'hide_pet_bubble') return null

    throw new Error(`Unexpected Tauri command in pet lifecycle test: ${command}`)
  })
}

describe('PetWindow lifecycle', () => {
  beforeEach(() => {
    mockWindows('pet')
    eventMocks.listeners.clear()
    eventMocks.unlisteners = []
    eventMocks.listen.mockReset()
    eventMocks.listen.mockImplementation(
      async (eventName: string, callback: (event: { payload: unknown }) => void) => {
        const unlisten = vi.fn(() => eventMocks.listeners.delete(eventName))
        eventMocks.listeners.set(eventName, callback)
        eventMocks.unlisteners.push(unlisten)
        return unlisten
      },
    )
    eventMocks.emit.mockClear()
    shortcutMocks.register.mockClear()
    shortcutMocks.unregister.mockClear()
  })

  it.each(['light', 'animal-island'] as const)(
    'registers and releases window resources in the %s theme',
    async (theme) => {
      installPetWindowCommands(theme)
      const wrapper = mount(PetWindow, {
        global: {
          stubs: {
            Pet: { template: '<span class="pet-stub" />' },
          },
        },
      })
      await flushPromises()

      expect(wrapper.get('.pet-window').classes()).toContain(`theme-${theme}`)
      expect([...eventMocks.listeners.keys()]).toEqual(
        expect.arrayContaining([
          'pet-skin-updated',
          'pet-animation-state',
          'codex-status-updated',
          'ui-theme-changed',
          'pet-action-bindings-changed',
          'pet-bubble-item-clicked',
          'pet-bubble-item-hover-changed',
        ]),
      )
      expect(shortcutMocks.register).toHaveBeenCalledWith('CommandOrControl+Alt+T', expect.any(Function))

      wrapper.unmount()
      await flushPromises()

      expect(eventMocks.unlisteners.length).toBeGreaterThanOrEqual(7)
      eventMocks.unlisteners.forEach((unlisten) => expect(unlisten).toHaveBeenCalledOnce())
      expect(eventMocks.listeners.size).toBe(0)
      expect(shortcutMocks.unregister).toHaveBeenCalledWith('CommandOrControl+Alt+T')
      expect(tauriCommandMock).toHaveBeenCalledWith('hide_pet_bubble', {})
    },
  )
})
