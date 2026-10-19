import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TranslatorApiSettings from './TranslatorApiSettings.vue'
import { tauriCommandMock } from '../tests/tauriBridgeMock'
import type { TranslationSettingsDraft, TranslationSettingsView } from '../types/app'

function view(): TranslationSettingsView {
  return { useGlobalApi: false, provider: 'custom', baseUrl: 'https://translation.example.test/v1', model: 'test-model', hasApiKey: true,
    effective: { useGlobalApi: false, enabled: true, provider: 'custom', model: 'test-model', host: 'translation.example.test', problem: null } }
}

describe('TranslatorApiSettings', () => {
  it.each(['keep', 'replace', 'clear'] as const)('tests the %s draft without saving or exposing the stored key', async (action) => {
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_translation_settings') return view()
      if (command === 'test_translation_connection') return { ok: true, message: '连接成功，当前草稿未保存。' }
      throw new Error(`Unexpected: ${command}`)
    })
    const wrapper = mount(TranslatorApiSettings)
    await flushPromises()
    expect(wrapper.get<HTMLInputElement>('input[type="password"]').element.value).toBe('')
    expect(wrapper.text()).toContain('已设置密钥')
    if (action === 'replace') await wrapper.get('input[type="password"]').setValue('replacement-placeholder')
    if (action === 'clear') await wrapper.get('input[type="checkbox"]').setValue(true)
    const test = wrapper.findAll('button').find((button) => button.text() === '测试连接')!
    await test.trigger('click')
    await flushPromises()
    expect(tauriCommandMock).toHaveBeenCalledWith('test_translation_connection', { settings: {
      useGlobalApi: false, provider: 'custom', baseUrl: 'https://translation.example.test/v1', model: 'test-model',
      keyAction: action, apiKey: action === 'replace' ? 'replacement-placeholder' : '',
    } })
    expect(wrapper.text()).toContain('当前草稿未保存')
    expect(wrapper.emitted('saved')).toBeUndefined()
    expect(tauriCommandMock.mock.calls.some(([command]) => command === 'save_translation_settings')).toBe(false)
    wrapper.unmount()
  })

  it('saves only translation fields and preserves independent draft on global mode switch', async () => {
    tauriCommandMock.mockImplementation(async (command: string, payload: { settings?: TranslationSettingsDraft }) => {
      if (command === 'get_translation_settings') return view()
      if (command === 'save_translation_settings') return { ...view(), useGlobalApi: payload.settings?.useGlobalApi }
      throw new Error(`Unexpected: ${command}`)
    })
    const wrapper = mount(TranslatorApiSettings)
    await flushPromises()
    const mode = wrapper.get('select')
    await mode.setValue('true')
    expect(wrapper.get('fieldset').element.disabled).toBe(true)
    await mode.setValue('false')
    expect(wrapper.get<HTMLInputElement>('input[type="text"]').element.value).toBe('https://translation.example.test/v1')
    await wrapper.findAll('button').find((button) => button.text() === '保存')!.trigger('click')
    await flushPromises()
    const payload = tauriCommandMock.mock.calls.find(([command]) => command === 'save_translation_settings')![1]
    expect(Object.keys(payload)).toEqual(['settings'])
    expect(payload.settings.keyAction).toBe('keep')
    expect(payload.settings).not.toHaveProperty('ai')
    expect(wrapper.emitted('saved')).toHaveLength(1)
    wrapper.unmount()
  })

  it('keeps the dialog and draft on save failure without announcing success', async () => {
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_translation_settings') return view()
      throw new Error('翻译设置保存失败，原配置未修改')
    })
    const wrapper = mount(TranslatorApiSettings)
    await flushPromises()
    await wrapper.get('input[type="password"]').setValue('replacement-placeholder')
    await wrapper.findAll('button').find((button) => button.text() === '保存')!.trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('原配置未修改')
    expect(wrapper.emitted('saved')).toBeUndefined()
    expect(wrapper.get<HTMLInputElement>('input[type="password"]').element.value).toBe('replacement-placeholder')
    wrapper.unmount()
  })

  it('traps Tab and Shift+Tab within the dialog and cancels without saving', async () => {
    tauriCommandMock.mockResolvedValue(view())
    const wrapper = mount(TranslatorApiSettings, { attachTo: document.body })
    await flushPromises()
    const first = wrapper.get('header button').element as HTMLButtonElement
    const footerButtons = wrapper.findAll('footer button')
    const last = footerButtons[footerButtons.length - 1].element as HTMLButtonElement
    first.focus()
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Tab', shiftKey: true })
    expect(document.activeElement).toBe(last)
    await wrapper.get('[role="dialog"]').trigger('keydown', { key: 'Tab' })
    expect(document.activeElement).toBe(first)
    await wrapper.findAll('button').find((button) => button.text() === '取消')!.trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
    expect(tauriCommandMock.mock.calls.some(([command]) => command === 'save_translation_settings')).toBe(false)
    wrapper.unmount()
  })

  it('announces read/test errors and offers a read retry', async () => {
    let failRead = true
    tauriCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_translation_settings' && !failRead) return view()
      throw new Error('离线模拟失败')
    })
    const wrapper = mount(TranslatorApiSettings)
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('离线模拟失败')
    failRead = false
    await wrapper.findAll('button').find((button) => button.text() === '重试读取')!.trigger('click')
    await flushPromises()
    await wrapper.findAll('button').find((button) => button.text() === '测试连接')!.trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('离线模拟失败')
    wrapper.unmount()
  })
})
