import { flushPromises, mount } from '@vue/test-utils'
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

describe('TranslatorWindow', () => {
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
      expect(notice.text()).toContain('AI 接口未启用')
      expect(notice.text()).toContain('请先在抽屉设置中启用 AI 接口')
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
})
