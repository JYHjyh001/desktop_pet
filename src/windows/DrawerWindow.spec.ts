import { flushPromises, shallowMount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DrawerWindow from './DrawerWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

function drawerConfig(theme: 'light' | 'animal-island' = 'animal-island') {
  return {
    drawer: {
      alwaysOnTop: false,
      categories: [],
      quickSearchTags: [],
      tagDisplayMode: 'compact',
      theme,
    },
    pet: {
      alwaysOnTop: true,
      size: 160,
    },
    ai: {
      enabled: false,
      provider: 'custom',
      apiKey: '',
      baseUrl: 'https://gateway.example.test/v1',
      model: 'test-model',
      profiles: [
        {
          id: 'test-profile',
          label: '测试连接',
          provider: 'custom',
          apiKey: '',
          baseUrl: 'https://gateway.example.test/v1',
          model: 'test-model',
        },
      ],
      activeProfileId: 'test-profile',
    },
  }
}

function installDrawerCommands(theme: 'light' | 'animal-island' = 'animal-island') {
  const testSkin = {
    id: 'test-skin',
    name: '测试宠物',
    builtin: true,
    preview: null,
    animations: {},
  }
  tauriCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_config') return drawerConfig(theme)
    if (command === 'get_apps') return []
    if (command === 'list_pet_skins') return [testSkin]
    if (command === 'get_current_pet_skin') return testSkin
    if (command === 'list_companions') return []
    if (command === 'get_current_companion') return null
    if (command === 'list_pet_memories') return []
    if (command === 'get_codex_app_server_status') return null
    if (command === 'get_storage_settings') {
      return { dataDir: '', memoryDir: '', petAssetsDir: '', iconsDir: '' }
    }
    if (command === 'get_runtime_info') {
      return {
        version: 'test',
        executablePath: '',
        defaultDataDir: '',
        dataDir: '',
        memoryDir: '',
        petAssetsDir: '',
        iconsDir: '',
        storageConfigFile: '',
      }
    }
    if (command === 'check_for_update') {
      return { currentVersion: 'test', status: 'latest', message: '已是最新版本' }
    }

    throw new Error(`Unexpected Tauri command in drawer smoke test: ${command}`)
  })
}

describe('DrawerWindow accessibility', () => {
  it('supports settings tabs and restores focus after Escape closes dialogs', async () => {
    installDrawerCommands()
    const wrapper = shallowMount(DrawerWindow, {
      attachTo: document.body,
      global: {
        stubs: {
          AppCard: true,
          CategoryList: true,
          SearchBar: true,
        },
      },
    })
    await flushPromises()

    const settingsTrigger = wrapper
      .findAll('button')
      .find((button) => button.text().trim() === '设置')
    expect(settingsTrigger).toBeDefined()
    const settingsTriggerElement = settingsTrigger!.element as HTMLButtonElement
    settingsTriggerElement.focus()
    await settingsTrigger!.trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 30))
    await flushPromises()

    const settingsDialog = wrapper.get('[role="dialog"][aria-labelledby="drawer-settings-title"]')
    expect(settingsDialog.attributes('aria-modal')).toBe('true')
    expect(wrapper.get('.drawer-window').classes()).toContain('theme-animal-island')

    const tabs = wrapper.findAll('[role="tab"]')
    expect(tabs.length).toBeGreaterThan(2)
    expect(tabs[0].attributes('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(tabs[0].element)

    await tabs[0].trigger('keydown', { key: 'ArrowDown' })
    expect(tabs[1].attributes('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(tabs[1].element)
    expect(wrapper.get('[role="tabpanel"]').attributes('aria-labelledby')).toBe(tabs[1].attributes('id'))

    const aiTab = tabs.find((tab) => tab.text().includes('AI 接口'))
    expect(aiTab).toBeDefined()
    await aiTab!.trigger('click')
    const deleteProfileButton = wrapper.get<HTMLButtonElement>('button[title="删除配置标签"]')
    deleteProfileButton.element.focus()
    await deleteProfileButton.trigger('click')
    await flushPromises()

    const confirmation = wrapper.get('[role="alertdialog"]')
    expect(confirmation.attributes('aria-labelledby')).toBe('important-confirmation-title')
    const cancelButton = confirmation.findAll('button').find((button) => button.text() === '取消')
    expect(cancelButton).toBeDefined()
    expect(document.activeElement).toBe(cancelButton!.element)

    await confirmation.trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
    expect(document.activeElement).toBe(deleteProfileButton.element)

    await settingsDialog.trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(wrapper.find('[role="dialog"][aria-labelledby="drawer-settings-title"]').exists()).toBe(false)
    expect(document.activeElement).toBe(settingsTriggerElement)

    wrapper.unmount()
  })

  it.each(['light', 'animal-island'] as const)(
    'gives entry and pet skin dialogs focus semantics in the %s theme',
    async (theme) => {
      installDrawerCommands(theme)
      const wrapper = shallowMount(DrawerWindow, {
        attachTo: document.body,
        global: {
          stubs: {
            AppCard: true,
            CategoryList: true,
            SearchBar: {
              emits: ['add'],
              template:
                '<button type="button" class="test-add-entry" @click="$emit(\'add\', \'app\')">添加入口</button>',
            },
          },
        },
      })
      await flushPromises()

      expect(wrapper.get('.drawer-window').classes()).toContain(`theme-${theme}`)

      const addEntryButton = wrapper.get<HTMLButtonElement>('.test-add-entry')
      addEntryButton.element.focus()
      await addEntryButton.trigger('click')
      await flushPromises()

      const entryDialog = wrapper.get(
        '[role="dialog"][aria-labelledby="drawer-entry-modal-title"]',
      )
      expect(entryDialog.attributes('aria-describedby')).toBe('drawer-entry-kind-options')
      expect(document.activeElement).toBe(entryDialog.get('input').element)

      await entryDialog.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="drawer-entry-modal-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(addEntryButton.element)

      const petSkinTrigger = wrapper
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '更换')
      expect(petSkinTrigger).toBeDefined()
      petSkinTrigger!.element.focus()
      await petSkinTrigger!.trigger('click')
      await flushPromises()

      const petSkinDialog = wrapper.get(
        '[role="dialog"][aria-labelledby="pet-skin-modal-title"]',
      )
      expect(petSkinDialog.attributes('aria-describedby')).toBe('pet-skin-modal-description')
      expect(document.activeElement).toBe(
        petSkinDialog.get<HTMLButtonElement>('[aria-label="关闭宠物形象管理"]').element,
      )

      await petSkinDialog.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="pet-skin-modal-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(petSkinTrigger!.element)

      wrapper.unmount()
    },
  )
})
