import { computed, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppDraft, PetApp } from '../types/app'

const apps = ref<PetApp[]>([])
const keyword = ref('')
const category = ref('全部')
const loading = ref(false)
const error = ref('')

const defaultCategories = ['全部', '常用', '开发工具', '游戏', '办公', '系统工具', '其他']
const configuredCategories = ref<string[]>(defaultCategories)

function normalizeCategories(categories: string[]) {
  const normalized = categories.map((item) => item.trim()).filter(Boolean)
  const output = ['全部', '常用']

  for (const category of normalized) {
    if (!output.some((item) => item.toLowerCase() === category.toLowerCase())) {
      output.push(category)
    }
  }

  if (!output.includes('其他')) {
    output.push('其他')
  }

  return output
}

function normalizeApp(app: PetApp): PetApp {
  return {
    ...app,
    icon: app.icon ?? null,
    iconDataUrl: app.iconDataUrl ?? null,
    launchCount: app.launchCount ?? 0,
    launchHistory: app.launchHistory ?? [],
    tags: app.tags ?? [],
    favorite: Boolean(app.favorite),
    autoFavorite: Boolean(app.autoFavorite),
  }
}

async function attachIconData(app: PetApp): Promise<PetApp> {
  const normalized = normalizeApp(app)

  if (!normalized.icon) {
    return normalized
  }

  try {
    const iconDataUrl = await invoke<string>('get_image_data_url', {
      relativePath: normalized.icon,
    })

    return {
      ...normalized,
      iconDataUrl,
    }
  } catch {
    return normalized
  }
}

export function useAppStore() {
  const categories = computed(() => {
    const custom = apps.value.map((app) => app.category).filter(Boolean)
    return Array.from(new Set([...configuredCategories.value, ...custom]))
  })

  const filteredApps = computed(() => {
    const query = keyword.value.trim().toLowerCase()

    return apps.value.filter((app) => {
      const matchCategory =
        category.value === '全部' ||
        (category.value === '常用' && app.favorite) ||
        app.category === category.value

      const searchText = [app.name, app.category, ...app.tags].join(' ').toLowerCase()
      const matchKeyword = !query || searchText.includes(query)

      return matchCategory && matchKeyword
    })
  })

  async function loadApps() {
    loading.value = true
    error.value = ''

    try {
      const result = await invoke<PetApp[]>('get_apps')
      apps.value = await Promise.all(result.map(attachIconData))
    } catch (err) {
      error.value = String(err)
    } finally {
      loading.value = false
    }
  }

  async function upsertApp(draft: AppDraft) {
    const result = await invoke<PetApp>('upsert_app', { draft })
    const app = await attachIconData(result)
    const index = apps.value.findIndex((item) => item.id === app.id)

    if (index >= 0) {
      apps.value.splice(index, 1, app)
    } else {
      apps.value.unshift(app)
    }
  }

  async function removeApp(appId: string) {
    await invoke('delete_app', { appId })
    apps.value = apps.value.filter((app) => app.id !== appId)
  }

  async function launchApp(appId: string) {
    await invoke('launch_app', { appId })
    await loadApps()
  }

  async function openAppDirectory(appId: string) {
    await invoke('open_app_dir', { appId })
  }

  function setConfiguredCategories(categories: string[]) {
    const nextCategories = normalizeCategories(categories.length > 0 ? categories : defaultCategories)
    configuredCategories.value = nextCategories
    const availableCategories = new Set([
      ...nextCategories,
      ...apps.value.map((app) => app.category).filter(Boolean),
    ])

    if (!availableCategories.has(category.value)) {
      category.value = '全部'
    }
  }

  return reactive({
    apps,
    keyword,
    category,
    categories,
    filteredApps,
    loading,
    error,
    loadApps,
    upsertApp,
    removeApp,
    launchApp,
    openAppDirectory,
    setConfiguredCategories,
  })
}
