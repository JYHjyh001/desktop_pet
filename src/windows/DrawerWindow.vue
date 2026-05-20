<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as emitEvent } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import AppCard from '../components/AppCard.vue'
import CategoryList from '../components/CategoryList.vue'
import SearchBar from '../components/SearchBar.vue'
import { useAppStore } from '../stores/appStore'
import type {
  AppDraft,
  PetAnimationSet,
  PetApp,
  PetDrawerConfig,
  PetSkinSummary,
  UpdateCheckResult,
} from '../types/app'
import { defaultPetAnimations, defaultPetPreview } from '../utils/defaultPet'
import { appNameFromPath, parseTags } from '../utils/format'

const store = useAppStore()
const modalVisible = ref(false)
const saving = ref(false)
const iconLoading = ref(false)
const formError = ref('')
const drawerWindow = getCurrentWindow()
const petSkinModalVisible = ref(false)
const settingsModalVisible = ref(false)
const petSkins = ref<PetSkinSummary[]>([])
const currentPetSkin = ref<PetSkinSummary | null>(null)
const selectedPetSkin = ref<PetSkinSummary | null>(null)
const petSkinLoading = ref(false)
const petSkinError = ref('')
const skinImporting = ref(false)
const quickSearchTags = ref<string[]>([])
const tagDisplayMode = ref<'compact' | 'detailed'>('compact')
const settingsSaving = ref(false)
const settingsError = ref('')
const updateChecking = ref(false)
const updateInfo = ref<UpdateCheckResult | null>(null)
const updateError = ref('')

const skinDraft = reactive({
  name: '',
  idle: '',
  hover: '',
  dragging: '',
  click: '',
})

const animationFields: Array<{
  key: keyof PetAnimationSet
  label: string
  required?: boolean
}> = [
  { key: 'idle', label: '待机动画', required: true },
  { key: 'hover', label: '选中动画' },
  { key: 'click', label: '点击动画' },
  { key: 'dragging', label: '拖动动画' },
]

const form = reactive({
  id: '',
  name: '',
  path: '',
  icon: '',
  iconPreview: '',
  category: '其他',
  tags: '',
  favorite: false,
})

const settingsDraft = reactive({
  categories: [] as string[],
  quickSearchTags: [] as string[],
  newCategory: '',
  newQuickTag: '',
  tagDisplayMode: 'compact' as 'compact' | 'detailed',
  petAlwaysOnTop: true,
  drawerAlwaysOnTop: true,
})

const isEditing = computed(() => Boolean(form.id))

onMounted(() => {
  void store.loadApps()
  void loadPetSkins()
  void loadDrawerSettings()
})

async function loadDrawerSettings() {
  try {
    const config = await invoke<PetDrawerConfig>('get_config')
    quickSearchTags.value = config.drawer.quickSearchTags ?? []
    tagDisplayMode.value = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
    store.setConfiguredCategories(config.drawer.categories ?? [])
    syncSettingsDraft(config)
  } catch (err) {
    console.error(err)
    quickSearchTags.value = []
    tagDisplayMode.value = 'compact'
    store.setConfiguredCategories([])
  }
}

function applyQuickSearchTag(tag: string) {
  store.keyword = tag
}

async function openSettings() {
  settingsModalVisible.value = true
  settingsError.value = ''
  updateError.value = ''
  await Promise.all([loadDrawerSettings(), checkForUpdate()])
}

function syncSettingsDraft(config: PetDrawerConfig) {
  settingsDraft.categories = [...(config.drawer.categories ?? [])]
  settingsDraft.quickSearchTags = [...(config.drawer.quickSearchTags ?? [])]
  settingsDraft.newCategory = ''
  settingsDraft.newQuickTag = ''
  settingsDraft.tagDisplayMode = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
  settingsDraft.petAlwaysOnTop = config.pet.alwaysOnTop
  settingsDraft.drawerAlwaysOnTop = config.drawer.alwaysOnTop
}

function normalizeTagDisplayMode(value?: string | null): 'compact' | 'detailed' {
  return value === 'detailed' ? 'detailed' : 'compact'
}

function addSettingsCategory() {
  const category = settingsDraft.newCategory.trim()
  settingsError.value = ''

  if (!category) {
    settingsError.value = '请输入分类名称'
    return
  }

  if (settingsDraft.categories.some((item) => item.toLowerCase() === category.toLowerCase())) {
    settingsError.value = '分类已存在'
    return
  }

  settingsDraft.categories.push(category)
  settingsDraft.newCategory = ''
}

function removeSettingsCategory(category: string) {
  if (isCoreCategory(category)) {
    return
  }

  settingsDraft.categories = settingsDraft.categories.filter((item) => item !== category)
}

function isCoreCategory(category: string) {
  return ['全部', '常用', '其他'].includes(category)
}

function addSettingsQuickTag() {
  const tag = settingsDraft.newQuickTag.trim()
  settingsError.value = ''

  if (!tag) {
    settingsError.value = '请输入快捷搜索标签'
    return
  }

  if (settingsDraft.quickSearchTags.some((item) => item.toLowerCase() === tag.toLowerCase())) {
    settingsError.value = '快捷搜索标签已存在'
    return
  }

  settingsDraft.quickSearchTags.push(tag)
  settingsDraft.newQuickTag = ''
}

function removeSettingsQuickTag(tag: string) {
  settingsDraft.quickSearchTags = settingsDraft.quickSearchTags.filter((item) => item !== tag)
}

async function saveSettings() {
  settingsSaving.value = true
  settingsError.value = ''

  try {
    const config = await invoke<PetDrawerConfig>('save_drawer_preferences', {
      preferences: {
        categories: [...settingsDraft.categories],
        quickSearchTags: [...settingsDraft.quickSearchTags],
        tagDisplayMode: settingsDraft.tagDisplayMode,
        petAlwaysOnTop: settingsDraft.petAlwaysOnTop,
        drawerAlwaysOnTop: settingsDraft.drawerAlwaysOnTop,
      },
    })

    quickSearchTags.value = config.drawer.quickSearchTags ?? []
    tagDisplayMode.value = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
    store.setConfiguredCategories(config.drawer.categories ?? [])
    syncSettingsDraft(config)
    settingsModalVisible.value = false
  } catch (err) {
    settingsError.value = String(err)
  } finally {
    settingsSaving.value = false
  }
}

async function checkForUpdate() {
  updateChecking.value = true
  updateError.value = ''

  try {
    updateInfo.value = await invoke<UpdateCheckResult>('check_for_update')
  } catch (err) {
    updateError.value = String(err)
  } finally {
    updateChecking.value = false
  }
}

async function openUpdatePage() {
  updateError.value = ''

  try {
    await invoke('open_update_page', { url: updateInfo.value?.updateUrl ?? null })
  } catch (err) {
    updateError.value = String(err)
  }
}

async function loadPetSkins() {
  petSkinLoading.value = true
  petSkinError.value = ''

  try {
    const [skins, current] = await Promise.all([
      invoke<PetSkinSummary[]>('list_pet_skins'),
      invoke<PetSkinSummary>('get_current_pet_skin'),
    ])
    petSkins.value = skins
    currentPetSkin.value = current
    selectedPetSkin.value =
      skins.find((skin) => skin.id === selectedPetSkin.value?.id) ??
      skins.find((skin) => skin.id === current.id) ??
      current
  } catch (err) {
    console.error(err)
    petSkinError.value = String(err)
  } finally {
    petSkinLoading.value = false
  }
}

async function openPetSkinManager() {
  petSkinModalVisible.value = true
  await loadPetSkins()
}

async function startDrawerDrag(event: PointerEvent) {
  if (event.button !== 0) {
    return
  }

  const target = event.target
  if (target instanceof HTMLElement && target.closest('button, input, textarea, select')) {
    return
  }

  try {
    await drawerWindow.startDragging()
  } catch (err) {
    console.error(err)
  }
}

function resetForm() {
  form.id = ''
  form.name = ''
  form.path = ''
  form.icon = ''
  form.iconPreview = ''
  form.category = '其他'
  form.tags = ''
  form.favorite = false
  formError.value = ''
}

function openAddModal() {
  resetForm()
  modalVisible.value = true
}

function openEditModal(app: PetApp) {
  form.id = app.id
  form.name = app.name
  form.path = app.path
  form.icon = app.icon ?? ''
  form.iconPreview = app.iconDataUrl ?? ''
  form.category = app.category
  form.tags = app.tags.join('，')
  form.favorite = app.favorite
  formError.value = ''
  modalVisible.value = true
}

async function pickExecutable() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: 'Windows 可执行程序',
        extensions: ['exe'],
      },
    ],
  })

  if (typeof selected === 'string') {
    form.path = selected
    if (!form.name) {
      form.name = appNameFromPath(selected)
    }
    await autoFillAppIcon(selected)
  }
}

async function autoFillAppIcon(path: string) {
  iconLoading.value = true

  try {
    const relativePath = await invoke<string>('import_executable_icon', { path })
    form.icon = relativePath
    form.iconPreview = await invoke<string>('get_image_data_url', { relativePath })
  } catch (err) {
    console.warn('自动获取软件图标失败', err)
  } finally {
    iconLoading.value = false
  }
}

async function selectPetSkin(skin: PetSkinSummary) {
  selectedPetSkin.value = skin

  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('set_pet_skin', { skinId: skin.id })
    selectedPetSkin.value = currentPetSkin.value
    await emitEvent('pet-skin-updated', skin.id)
    await loadPetSkins()
  } catch (err) {
    petSkinError.value = String(err)
  }
}

async function pickPetAnimation(state: keyof PetAnimationSet) {
  const selected = await openImageFile()
  if (typeof selected !== 'string') {
    return
  }

  skinDraft[state] = selected

  if (state === 'idle' && !skinDraft.name.trim()) {
    skinDraft.name = appNameFromPath(selected)
  }
}

function clearPetAnimation(state: keyof PetAnimationSet) {
  skinDraft[state] = ''
}

async function importPetSkin() {
  if (!skinDraft.idle) {
    petSkinError.value = '导入宠物至少需要选择待机动画'
    return
  }

  skinImporting.value = true
  petSkinError.value = ''

  try {
    const imported = await invoke<PetSkinSummary>('import_pet_skin', {
      name: skinDraft.name.trim() || '自定义宠物',
      animations: {
        idle: skinDraft.idle,
        hover: skinDraft.hover || null,
        dragging: skinDraft.dragging || null,
        click: skinDraft.click || null,
      },
    })

    currentPetSkin.value = imported
    selectedPetSkin.value = imported
    skinDraft.name = ''
    skinDraft.idle = ''
    skinDraft.hover = ''
    skinDraft.dragging = ''
    skinDraft.click = ''
    await emitEvent('pet-skin-updated', imported.id)
    await loadPetSkins()
  } catch (err) {
    petSkinError.value = String(err)
  } finally {
    skinImporting.value = false
  }
}

async function resetPetImage() {
  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('set_pet_skin', { skinId: 'default' })
    selectedPetSkin.value = currentPetSkin.value
    await emitEvent('pet-skin-updated', 'default')
    await loadPetSkins()
  } catch (err) {
    alert(`恢复默认宠物失败：${String(err)}`)
  }
}

async function pickAppIcon() {
  const selected = await openImageFile()

  if (typeof selected !== 'string') {
    return
  }

  try {
    const relativePath = await invoke<string>('import_app_icon', { path: selected })
    form.icon = relativePath
    form.iconPreview = await invoke<string>('get_image_data_url', { relativePath })
  } catch (err) {
    formError.value = String(err)
  }
}

async function openImageFile() {
  return open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: '图片',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'ico'],
      },
    ],
  })
}

async function saveApp() {
  formError.value = ''

  if (!form.name.trim()) {
    formError.value = '请填写软件名称'
    return
  }

  if (!form.path.trim()) {
    formError.value = '请选择或填写软件路径'
    return
  }

  saving.value = true

  const draft: AppDraft = {
    id: form.id || undefined,
    name: form.name.trim(),
    path: form.path.trim(),
    icon: form.icon || undefined,
    category: form.category.trim() || '其他',
    tags: parseTags(form.tags),
    favorite: form.favorite,
  }

  try {
    await store.upsertApp(draft)
    modalVisible.value = false
  } catch (err) {
    formError.value = String(err)
  } finally {
    saving.value = false
  }
}

async function removeApp(app: PetApp) {
  if (!confirm(`确认删除 ${app.name}？`)) {
    return
  }

  await store.removeApp(app.id)
}

async function launchApp(app: PetApp) {
  try {
    await store.launchApp(app.id)
  } catch (err) {
    alert(`启动失败：${String(err)}`)
  }
}

async function openAppDirectory(app: PetApp) {
  try {
    await store.openAppDirectory(app.id)
  } catch (err) {
    alert(`打开目录失败：${String(err)}`)
  }
}

async function hideDrawer() {
  await invoke('hide_drawer')
}
</script>

<template>
  <main class="drawer-window">
    <header class="drawer-header" @pointerdown="startDrawerDrag">
      <div class="drawer-titlebar">
        <h1>PetDrawer</h1>
        <p>桌面宠物软件抽屉</p>
      </div>
      <div class="header-actions">
        <button class="secondary-button" type="button" @click="openSettings">设置</button>
        <button class="window-close" type="button" title="隐藏抽屉" @click="hideDrawer">×</button>
      </div>
    </header>

    <section class="drawer-layout">
      <aside class="drawer-sidebar">
        <section class="pet-preview-panel">
          <div class="pet-preview-frame">
            <img :src="currentPetSkin?.preview || defaultPetPreview" alt="" />
          </div>
          <div class="pet-preview-copy">
            <h2>当前宠物形象</h2>
            <p v-if="currentPetSkin">{{ currentPetSkin.name }}</p>
            <p v-else>内置默认形象</p>
          </div>
          <div class="pet-preview-actions">
            <button class="secondary-button" type="button" @click="openPetSkinManager">
              更换
            </button>
            <button class="secondary-button" type="button" @click="resetPetImage">默认</button>
          </div>
        </section>

        <CategoryList
          :categories="store.categories"
          :active="store.category"
          @select="store.category = $event"
        />

      </aside>

      <section class="drawer-main">
        <SearchBar
          v-model="store.keyword"
          :quick-tags="quickSearchTags"
          @quick-tag="applyQuickSearchTag"
          @add="openAddModal"
        />

        <div class="app-panel">
          <div class="panel-status" v-if="store.loading">正在读取软件列表...</div>
          <div class="panel-status error" v-else-if="store.error">{{ store.error }}</div>
          <div class="empty-state" v-else-if="store.filteredApps.length === 0">
            <h2>还没有匹配的软件</h2>
            <p>添加一个本地 exe 文件后，它会保存在本机 JSON 数据中。</p>
            <button class="primary-button" type="button" @click="openAddModal">添加软件</button>
          </div>
          <div class="app-grid" :class="{ compact: tagDisplayMode === 'compact' }" v-else>
            <AppCard
              v-for="app in store.filteredApps"
              :key="app.id"
              :app="app"
              :tag-display-mode="tagDisplayMode"
              @launch="launchApp"
              @edit="openEditModal"
              @remove="removeApp"
              @open-dir="openAppDirectory"
            />
          </div>
        </div>
      </section>
    </section>

    <div v-if="modalVisible" class="modal-backdrop" @click.self="modalVisible = false">
      <form class="app-modal" @submit.prevent="saveApp">
        <header>
          <h2>{{ isEditing ? '编辑软件' : '添加软件' }}</h2>
          <button type="button" class="window-close" @click="modalVisible = false">×</button>
        </header>

        <label>
          软件名称
          <input v-model="form.name" autocomplete="off" />
        </label>

        <label>
          软件路径
          <div class="path-row">
            <input v-model="form.path" autocomplete="off" />
            <button type="button" @click="pickExecutable">选择</button>
          </div>
        </label>

        <label>
          软件图标
          <div class="icon-picker">
            <span class="icon-preview">
              <img v-if="form.iconPreview" :src="form.iconPreview" alt="" />
              <span v-else>默认</span>
            </span>
            <button type="button" :disabled="iconLoading" @click="pickAppIcon">
              {{ iconLoading ? '获取中...' : '选择图标' }}
            </button>
          </div>
        </label>

        <label>
          分类
          <input v-model="form.category" list="category-options" autocomplete="off" />
          <datalist id="category-options">
            <option v-for="category in store.categories" :key="category" :value="category" />
          </datalist>
        </label>

        <label>
          标签
          <input v-model="form.tags" placeholder="多个标签用空格或逗号分隔" autocomplete="off" />
        </label>

        <label class="checkbox-row">
          <input v-model="form.favorite" type="checkbox" />
          设为常用
        </label>

        <p v-if="formError" class="form-error">{{ formError }}</p>

        <footer>
          <button type="button" @click="modalVisible = false">取消</button>
          <button class="primary-button" type="submit" :disabled="saving">
            {{ saving ? '保存中...' : '保存' }}
          </button>
        </footer>
      </form>
    </div>

    <div
      v-if="petSkinModalVisible"
      class="modal-backdrop"
      @click.self="petSkinModalVisible = false"
    >
      <section class="skin-modal">
        <header>
          <div>
            <h2>更换宠物形象</h2>
            <p>从宠物形象文件夹选择，或导入新的多状态宠物。</p>
          </div>
          <button type="button" class="window-close" @click="petSkinModalVisible = false">
            ×
          </button>
        </header>

        <p v-if="petSkinError" class="form-error">{{ petSkinError }}</p>
        <div v-if="petSkinLoading" class="skin-loading">正在搜索宠物形象...</div>

        <div v-else class="skin-manager-layout">
          <div class="skin-grid">
            <button
              v-for="skin in petSkins"
              :key="skin.id"
              type="button"
              class="skin-card"
              :class="{
                active: currentPetSkin?.id === skin.id,
                selected: selectedPetSkin?.id === skin.id,
              }"
              @click="selectPetSkin(skin)"
            >
              <span class="skin-thumb">
                <img
                  v-if="skin.preview || skin.builtin"
                  :src="skin.preview || defaultPetPreview"
                  alt=""
                />
                <span v-else>默认</span>
              </span>
              <span class="skin-card-name">{{ skin.name }}</span>
              <span class="skin-state-tags">
                <span>待机</span>
                <span v-if="skin.animations.hover">选中</span>
                <span v-if="skin.animations.click">点击</span>
                <span v-if="skin.animations.dragging">拖动</span>
              </span>
            </button>
          </div>

          <aside class="skin-detail-panel">
            <div class="skin-detail-preview">
              <img
                v-if="selectedPetSkin?.preview || selectedPetSkin?.builtin"
                :src="selectedPetSkin?.preview || defaultPetPreview"
                alt=""
              />
              <span v-else>无预览</span>
            </div>
            <h3>{{ selectedPetSkin?.name || '未选择宠物' }}</h3>
            <p>{{ selectedPetSkin?.builtin ? '内置默认宠物' : '已存储宠物形象' }}</p>

            <div class="skin-animation-list" v-if="selectedPetSkin">
              <div v-for="field in animationFields" :key="field.key" class="skin-animation-item">
                <span class="animation-status-thumb">
                  <img
                    v-if="selectedPetSkin.animations[field.key]"
                    :src="selectedPetSkin.animations[field.key] || ''"
                    alt=""
                  />
                  <img
                    v-else-if="selectedPetSkin.builtin"
                    :src="defaultPetAnimations[field.key] || defaultPetPreview"
                    alt=""
                  />
                  <span v-else>回退</span>
                </span>
                <span>
                  <strong>{{ field.label }}</strong>
                  <small>
                    {{
                      selectedPetSkin.animations[field.key]
                        ? '已配置'
                        : selectedPetSkin.builtin
                          ? '内置'
                          : '使用待机动画'
                    }}
                  </small>
                </span>
              </div>
            </div>
          </aside>
        </div>

        <section class="skin-import-panel">
          <h3>导入宠物</h3>
          <label>
            宠物名称
            <input v-model="skinDraft.name" placeholder="例如：小猫助手" autocomplete="off" />
          </label>

          <div class="animation-picker-grid">
            <div v-for="field in animationFields" :key="field.key" class="animation-picker">
              <div>
                <strong>{{ field.label }}</strong>
                <span>{{ field.required ? '必填' : '可选，未设置时使用待机动画' }}</span>
                <p :title="skinDraft[field.key]">
                  {{ skinDraft[field.key] || '未选择图片' }}
                </p>
              </div>
              <div class="animation-picker-actions">
                <button type="button" @click="pickPetAnimation(field.key)">选择</button>
                <button
                  v-if="skinDraft[field.key]"
                  type="button"
                  @click="clearPetAnimation(field.key)"
                >
                  清除
                </button>
              </div>
            </div>
          </div>

          <footer>
            <button type="button" @click="petSkinModalVisible = false">关闭</button>
            <button class="primary-button" type="button" :disabled="skinImporting" @click="importPetSkin">
              {{ skinImporting ? '导入中...' : '导入并使用' }}
            </button>
          </footer>
        </section>
      </section>
    </div>

    <div
      v-if="settingsModalVisible"
      class="modal-backdrop"
      @click.self="settingsModalVisible = false"
    >
      <section class="settings-modal">
        <header>
          <div>
            <h2>设置</h2>
            <p>管理抽屉显示、分类、快捷搜索、窗口置顶和软件更新。</p>
          </div>
          <button type="button" class="window-close" @click="settingsModalVisible = false">
            ×
          </button>
        </header>

        <section class="settings-section">
          <h3>分类选项</h3>
          <form class="settings-add-row" @submit.prevent="addSettingsCategory">
            <input v-model="settingsDraft.newCategory" placeholder="输入分类名称" />
            <button class="primary-button" type="submit">添加</button>
          </form>
          <div class="settings-chip-list">
            <span
              v-for="category in settingsDraft.categories"
              :key="category"
              class="settings-chip"
              :class="{ locked: isCoreCategory(category) }"
            >
              {{ category }}
              <button
                v-if="!isCoreCategory(category)"
                type="button"
                title="删除分类"
                @click="removeSettingsCategory(category)"
              >
                ×
              </button>
            </span>
          </div>
        </section>

        <section class="settings-section">
          <h3>快捷搜索</h3>
          <form class="settings-add-row" @submit.prevent="addSettingsQuickTag">
            <input v-model="settingsDraft.newQuickTag" placeholder="输入搜索标签，如 VS Code、AI、办公" />
            <button class="primary-button" type="submit">添加</button>
          </form>
          <div class="settings-chip-list" v-if="settingsDraft.quickSearchTags.length > 0">
            <span v-for="tag in settingsDraft.quickSearchTags" :key="tag" class="settings-chip">
              {{ tag }}
              <button type="button" title="删除标签" @click="removeSettingsQuickTag(tag)">×</button>
            </span>
          </div>
          <p v-else class="settings-empty">还没有快捷搜索标签。</p>
        </section>

        <section class="settings-section">
          <h3>软件显示方式</h3>
          <div class="segmented-control">
            <button
              type="button"
              :class="{ active: settingsDraft.tagDisplayMode === 'compact' }"
              @click="settingsDraft.tagDisplayMode = 'compact'"
            >
              缩略显示
            </button>
            <button
              type="button"
              :class="{ active: settingsDraft.tagDisplayMode === 'detailed' }"
              @click="settingsDraft.tagDisplayMode = 'detailed'"
            >
              详细显示
            </button>
          </div>
        </section>

        <section class="settings-section">
          <h3>窗口置顶</h3>
          <label class="settings-toggle-row">
            <span>
              <strong>宠物置顶</strong>
              <small>宠物窗口保持在其他窗口上方</small>
            </span>
            <input v-model="settingsDraft.petAlwaysOnTop" type="checkbox" />
          </label>
          <label class="settings-toggle-row">
            <span>
              <strong>抽屉置顶</strong>
              <small>抽屉窗口打开后保持在其他窗口上方</small>
            </span>
            <input v-model="settingsDraft.drawerAlwaysOnTop" type="checkbox" />
          </label>
        </section>

        <section class="settings-section">
          <h3>软件更新</h3>
          <div class="settings-update-panel">
            <div>
              <strong>当前版本</strong>
              <small>
                v{{ updateInfo?.currentVersion || '读取中' }}
                <template v-if="updateInfo?.latestVersion">
                  / 最新 v{{ updateInfo.latestVersion }}
                </template>
              </small>
            </div>
            <div class="settings-update-actions">
              <button type="button" :disabled="updateChecking" @click="checkForUpdate">
                {{ updateChecking ? '检查中...' : '检查更新' }}
              </button>
              <button
                v-if="updateInfo?.updateUrl"
                type="button"
                class="primary-button"
                @click="openUpdatePage"
              >
                {{ updateInfo.status === 'available' ? '下载新版' : '打开发布页' }}
              </button>
            </div>
          </div>
          <p v-if="updateInfo?.assetName" class="settings-empty">
            下载文件：{{ updateInfo.assetName }}
          </p>
          <p v-if="updateInfo" class="settings-empty">{{ updateInfo.message }}</p>
          <p v-if="updateError" class="form-error">{{ updateError }}</p>
        </section>

        <p v-if="settingsError" class="form-error">{{ settingsError }}</p>

        <footer>
          <button type="button" @click="settingsModalVisible = false">取消</button>
          <button class="primary-button" type="button" :disabled="settingsSaving" @click="saveSettings">
            {{ settingsSaving ? '保存中...' : '保存设置' }}
          </button>
        </footer>
      </section>
    </div>
  </main>
</template>
