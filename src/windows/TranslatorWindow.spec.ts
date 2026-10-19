import { flushPromises, mount } from '@vue/test-utils'
import { emit } from '@tauri-apps/api/event'
import { describe, expect, it } from 'vitest'
import TranslatorWindow from './TranslatorWindow.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'

type TranslatorTheme = 'light' | 'animal-island'

interface TranslatorSmokeConfig {
  drawer: {
    theme: TranslatorTheme
  }
  ai: {
    enabled: boolean
    provider: string
    baseUrl: string
    model: string
  }
}

function createConfig(theme: TranslatorTheme, aiEnabled: boolean): TranslatorSmokeConfig {
  return {
    drawer: { theme },
    ai: {
      enabled: aiEnabled,
      provider: 'openai',
      baseUrl: 'https://gateway.example.test/v1',
      model: 'test-model',
    },
  }
}

function createSettings(enabled: boolean, useGlobalApi = true) {
  return { useGlobalApi, provider: 'custom', baseUrl: '', model: '', hasApiKey: false,
    effective: { useGlobalApi, enabled, provider: 'openai', model: 'test-model', host: 'gateway.example.test', problem: enabled ? null : '共用的全局 AI 接口未启用，请启用全局接口或设置独立翻译 API。' } }
}

describe('TranslatorWindow', () => {
  it('cancels transport and shows a safe cancellation result without losing the input', async () => {
    let rejectRequest!: (error: string) => void
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') return createConfig('light', true)
      if (command === 'get_translation_settings') return createSettings(true)
      if (command === 'translate_text') return new Promise((_resolve, reject) => { rejectRequest = reject })
      if (command === 'cancel_ai_requests') { rejectRequest?.('翻译请求已取消。'); return }
    })
    const wrapper = mount(TranslatorWindow)
    await flushPromises()
    await wrapper.get('textarea').setValue('保留原文')
    await wrapper.get('.translator-primary-button').trigger('click')
    await flushPromises()
    const cancel = wrapper.findAll('button').find((button) => button.text() === '取消请求')!
    await cancel.trigger('click')
    await flushPromises()
    expect(tauriCommandMock).toHaveBeenCalledWith('cancel_ai_requests', {})
    expect(wrapper.get('.translator-error').text()).toContain('已取消')
    expect(wrapper.get('textarea').element.value).toBe('保留原文')
    wrapper.unmount()
  })
  it.each([
    { theme: 'light' as const, aiEnabled: false },
    { theme: 'animal-island' as const, aiEnabled: false },
    { theme: 'light' as const, aiEnabled: true },
    { theme: 'animal-island' as const, aiEnabled: true },
  ])('renders $theme theme with AI enabled=$aiEnabled', async ({ theme, aiEnabled }) => {
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') {
        return createConfig(theme, aiEnabled)
      }
      if (command === 'get_translation_settings') return createSettings(aiEnabled)

      throw new Error(`Unexpected Tauri command in smoke test: ${command}`)
    })

    const wrapper = mount(TranslatorWindow)
    await flushPromises()

    expect(tauriCommandMock).toHaveBeenCalledWith('get_public_config', {})
    expect(wrapper.get('.translator-window').classes()).toContain(`theme-${theme}`)

    const notice = wrapper.get('.translator-notice')
    const translateButton = wrapper.get<HTMLButtonElement>('.translator-primary-button')
    expect(wrapper.get('.translator-output-panel small').attributes('aria-live')).toBe('polite')

    if (aiEnabled) {
      expect(notice.attributes('role')).toBe('note')
      expect(notice.attributes('aria-label')).toBe('AI 数据发送提示')
      expect(notice.text()).toContain('在线处理提示')
      expect(notice.text()).toContain('原文会发送到 gateway.example.test')
      expect(notice.text()).not.toContain('/v1')

      await wrapper.get('textarea').setValue('仅用于组件测试的文本')
      expect(translateButton.element.disabled).toBe(false)
    } else {
      expect(notice.attributes('role')).toBe('status')
      expect(notice.attributes('aria-live')).toBe('polite')
      expect(notice.text()).toContain('全局 AI 接口未启用')
      expect(notice.text()).toContain('独立翻译 API')
      expect(translateButton.element.disabled).toBe(true)
    }

    wrapper.unmount()
  })

  it('announces configuration loading errors to assistive technology', async () => {
    tauriCommandMock.mockRejectedValue(new Error('测试配置读取失败'))
    const wrapper = mount(TranslatorWindow)
    await flushPromises()

    const error = wrapper.get('.translator-error')
    expect(error.attributes('role')).toBe('alert')
    expect(error.attributes('aria-live')).toBe('assertive')
    expect(error.text()).toContain('测试配置读取失败')

    wrapper.unmount()
  })

  it.each(['light', 'animal-island'] as const)('translates with independent API and global AI disabled in %s', async (theme) => {
    const view = createSettings(true, false)
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') return createConfig(theme, false)
      if (command === 'get_translation_settings') return view
      if (command === 'translate_text') return { translatedText: '测试译文', provider: 'openai', model: 'test-model', service: view.effective }
      throw new Error(`Unexpected: ${command}`)
    })
    const wrapper = mount(TranslatorWindow)
    await flushPromises()
    expect(wrapper.get('.translator-header p').text()).toContain('独立翻译 API')
    await wrapper.get('textarea').setValue('测试原文')
    await wrapper.get('.translator-primary-button').trigger('click')
    await flushPromises()
    expect(tauriCommandMock).toHaveBeenCalledWith('translate_text', { request: { text: '测试原文', sourceLanguage: 'auto', targetLanguage: 'zh', mode: 'plain' } })
    expect(wrapper.get('.translator-output').text()).toBe('测试译文')
    expect(wrapper.get('.translator-output-panel small').text()).toContain('gateway.example.test')
    await wrapper.get('.translator-ghost-button').trigger('click')
    expect(wrapper.get('textarea').element.value).toBe('')
    wrapper.unmount()
  })

  it('refreshes effective connection before sending and does not send when it becomes unavailable', async () => {
    let enabled = true
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') return createConfig('light', true)
      if (command === 'get_translation_settings') return createSettings(enabled)
      throw new Error(`Unexpected: ${command}`)
    })
    const wrapper = mount(TranslatorWindow)
    await flushPromises()
    await wrapper.get('textarea').setValue('测试原文')
    enabled = false
    await wrapper.get('.translator-primary-button').trigger('click')
    await flushPromises()
    expect(tauriCommandMock.mock.calls.some(([command]) => command === 'translate_text')).toBe(false)
    expect(wrapper.get('.translator-error').text()).toContain('未启用')
    wrapper.unmount()
  })

  it.each(['light', 'animal-island'] as const)('opens settings and restores focus after Escape in %s', async (theme) => {
    tauriCommandMock.mockImplementation(async (command: string) => command === 'get_public_config' ? createConfig(theme, true) : createSettings(true))
    const wrapper = mount(TranslatorWindow, { attachTo: document.body })
    await flushPromises()
    const button = wrapper.findAll('.translator-header-actions button').find((item) => item.text() === '设置')!
    await button.trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="dialog"]').attributes('aria-modal')).toBe('true')
    expect(document.activeElement).toBe(wrapper.get('[role="dialog"] select').element)
    expect(wrapper.get('.translator-content').attributes('inert')).toBeDefined()
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
    expect(document.activeElement).toBe(button.element)
    expect(tauriCommandMock.mock.calls.some(([command]) => command === 'save_translation_settings')).toBe(false)
    wrapper.unmount()
  })

  it('updates the next-service summary on settings events while preserving the actual request result summary', async () => {
    let currentView = createSettings(true, false)
    const requestService = { ...currentView.effective }
    let complete: ((reply: unknown) => void) | undefined
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') return createConfig('light', true)
      if (command === 'get_translation_settings') return currentView
      if (command === 'translate_text') return new Promise((resolve) => { complete = resolve })
      throw new Error(`Unexpected: ${command}`)
    })
    const wrapper = mount(TranslatorWindow)
    await flushPromises()
    await wrapper.get('textarea').setValue('测试原文')
    await wrapper.get('.translator-primary-button').trigger('click')
    await flushPromises()
    currentView = createSettings(true, true)
    currentView.effective.host = 'next.example.test'
    currentView.effective.model = 'next-test-model'
    await emit('translation-config-changed')
    await flushPromises()
    expect(wrapper.get('.translator-header p').text()).toContain('共用全局 API')
    expect(wrapper.get('.translator-notice').text()).toContain('next.example.test')
    complete!({ translatedText: '测试译文', provider: 'openai', model: 'test-model', service: requestService })
    await flushPromises()
    expect(wrapper.get('.translator-output-panel small').text()).toContain('独立翻译 API')
    expect(wrapper.get('.translator-output-panel small').text()).toContain('gateway.example.test')
    expect(wrapper.get('.translator-notice').text()).toContain('next.example.test')
    wrapper.unmount()
  })

  it('preserves language, polish and copy controls and displays the actual selection translation service', async () => {
    const currentView = createSettings(true, true)
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_public_config') return createConfig('light', true)
      if (command === 'get_translation_settings') return currentView
      if (command === 'translate_text') return { translatedText: 'polished translation', provider: 'openai', model: 'test-model', service: currentView.effective }
      throw new Error(`Unexpected: ${command}`)
    })
    const copied: string[] = []
    const priorClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard')
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (text: string) => { copied.push(text) } } })
    const wrapper = mount(TranslatorWindow)
    await flushPromises()
    const languageSelects = wrapper.findAll('.translator-language-bar select')
    await languageSelects[0].setValue('en')
    await languageSelects[1].setValue('zh')
    await wrapper.findAll('.translator-mode-toggle button')[1].trigger('click')
    await wrapper.get('textarea').setValue('test input')
    await wrapper.get('.translator-primary-button').trigger('click')
    await flushPromises()
    expect(tauriCommandMock).toHaveBeenCalledWith('translate_text', { request: { text: 'test input', sourceLanguage: 'en', targetLanguage: 'zh', mode: 'polish' } })
    await wrapper.findAll('footer button').find((button) => button.text() === '复制译文')!.trigger('click')
    expect(copied).toEqual(['polished translation'])
    await emit('translator-prefill', { sourceText: 'selection input', translatedText: '划选译文', service: { ...currentView.effective, useGlobalApi: false, host: 'selection.example.test' } })
    await flushPromises()
    expect(wrapper.get('.translator-output').text()).toBe('划选译文')
    expect(wrapper.get('.translator-output-panel small').text()).toContain('selection.example.test')
    wrapper.unmount()
    if (priorClipboard) Object.defineProperty(navigator, 'clipboard', priorClipboard)
    else Reflect.deleteProperty(navigator, 'clipboard')
  })
})
