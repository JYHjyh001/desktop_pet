import { describe, expect, it } from 'vitest'
import type { PetApp } from '../types/app'
import { tauriCommandMock } from '../tests/tauriBridgeMock'
import { useAppStore } from './appStore'

function createApp(id: string, category: string): PetApp {
  return {
    id,
    name: id.toUpperCase(),
    itemKind: 'app',
    path: `${id}.exe`,
    category,
    runAsAdmin: false,
    tags: [],
    favorite: false,
    launchCount: 0,
    createdAt: '2026-08-01',
  }
}

function seedStore() {
  const store = useAppStore()
  store.keyword = ''
  store.category = '全部'
  store.setItemKindFilters([])
  store.apps.splice(0, store.apps.length, createApp('a', '开发工具'), createApp('b', '办公'), createApp('c', '开发工具'), createApp('d', '办公'))
  return store
}

describe('appStore shortcut ordering', () => {
  it('reorders only visible slots and persists the complete id sequence', async () => {
    const store = seedStore()
    store.category = '开发工具'
    tauriCommandMock.mockResolvedValue(undefined)

    await store.reorderApps('c', 'a')

    expect(store.apps.map((app) => app.id)).toEqual(['c', 'b', 'a', 'd'])
    expect(tauriCommandMock).toHaveBeenCalledWith('reorder_apps', {
      appIds: ['c', 'b', 'a', 'd'],
    })
  })

  it('rolls back the optimistic order when persistence fails', async () => {
    const store = seedStore()
    tauriCommandMock.mockRejectedValue(new Error('write failed'))

    await expect(store.reorderApps('a', 'c')).rejects.toThrow('write failed')
    expect(store.apps.map((app) => app.id)).toEqual(['a', 'b', 'c', 'd'])
  })
})
