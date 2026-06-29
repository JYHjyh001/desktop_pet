<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type {
  Companion,
  DrawerTheme,
  PetDrawerConfig,
  StoryCreateDraft,
  StoryMode,
  StorySave,
  StoryTurnReply,
} from '../types/app'

const storyWindow = getCurrentWindow()
const { windowOpenAnimationClass } = useWindowOpenAnimation('panel')
const config = ref<PetDrawerConfig | null>(null)
const companions = ref<Companion[]>([])
const storySaves = ref<StorySave[]>([])
const currentStory = ref<StorySave | null>(null)
const storyInput = ref('')
const errorMessage = ref('')
const storyLoading = ref(false)
const sending = ref(false)
const storyFullscreen = ref(false)
const storyLogRef = ref<HTMLElement | null>(null)
const storyDraft = ref({
  mode: 'random' as StoryMode,
  storyType: '随机',
  tone: '治愈',
  premise: '',
  companionIds: [] as string[],
  companionRole: '重要角色 / 搭档',
  temporaryName: '',
  temporaryIdentity: '',
  temporaryPersonality: '',
  temporaryRelationship: '',
  temporaryRole: '',
  temporaryHidden: '',
  temporaryInteractable: true,
})

let unlistenStoryOpened: (() => void) | null = null
let unlistenThemeChanged: (() => void) | null = null
let unlistenCompanionChanged: (() => void) | null = null
let unlistenStoryResized: (() => void) | null = null

const aiEnabled = computed(() => Boolean(config.value?.ai?.enabled))
const drawerTheme = computed<DrawerTheme>(() =>
  config.value?.drawer.theme === 'animal-island' ? 'animal-island' : 'light',
)
const aiSummary = computed(() => {
  const ai = config.value?.ai
  if (!ai?.enabled) {
    return '未启用 AI 接口'
  }
  return `${providerLabel(ai.provider)} / ${ai.model || '未设置模型'}`
})
const storyMessages = computed(() => currentStory.value?.recentMessages ?? [])
const latestAssistantMessage = computed(
  () => [...storyMessages.value].reverse().find((message) => message.role === 'assistant') ?? null,
)
const storyOptionActions = computed(() => extractStoryOptions(latestAssistantMessage.value?.content ?? ''))
const selectedCompanionNames = computed(() =>
  storyDraft.value.companionIds
    .map((id) => companions.value.find((companion) => companion.id === id)?.name)
    .filter((name): name is string => Boolean(name)),
)
const activeCharacterNames = computed(() =>
  currentStory.value?.characters
    .filter((character) => currentStory.value?.activeCharacterIds.includes(character.id) || character.isInteractable)
    .map((character) => character.name)
    .filter(Boolean)
    .slice(0, 6) ?? [],
)

onMounted(async () => {
  await refreshStoryWindow()
  await syncFullscreenState()
  unlistenStoryOpened = await listen('story-opened', () => {
    void refreshStoryWindow()
    void syncFullscreenState()
  })
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    if (config.value) {
      config.value.drawer.theme = event.payload
    }
  })
  unlistenCompanionChanged = await listen('companion-changed', () => {
    void loadCompanions()
  })
  unlistenStoryResized = await storyWindow.onResized(() => {
    void syncFullscreenState()
  })
})

onBeforeUnmount(() => {
  unlistenStoryOpened?.()
  unlistenThemeChanged?.()
  unlistenCompanionChanged?.()
  unlistenStoryResized?.()
})

watch(
  storyMessages,
  async () => {
    await nextTick()
    scrollStoryToBottom()
  },
  { deep: true },
)

async function refreshStoryWindow() {
  await Promise.all([loadConfig(), loadCompanions(), loadStorySaves()])
}

async function loadConfig() {
  try {
    config.value = await invoke<PetDrawerConfig>('get_config')
  } catch (err) {
    errorMessage.value = String(err)
  }
}

async function loadCompanions() {
  try {
    companions.value = await invoke<Companion[]>('list_companions')
    storyDraft.value.companionIds = storyDraft.value.companionIds.filter((id) =>
      companions.value.some((companion) => companion.id === id),
    )
  } catch (err) {
    errorMessage.value = String(err)
  }
}

async function loadStorySaves() {
  try {
    storySaves.value = await invoke<StorySave[]>('list_story_saves')
    if (currentStory.value) {
      currentStory.value = storySaves.value.find((story) => story.id === currentStory.value?.id) ?? currentStory.value
    }
  } catch (err) {
    errorMessage.value = String(err)
  }
}

async function startDrag() {
  if (storyFullscreen.value) {
    return
  }
  await storyWindow.startDragging()
}

async function hideStory() {
  if (storyFullscreen.value) {
    await storyWindow.setFullscreen(false)
    storyFullscreen.value = false
  }
  await invoke('hide_story')
}

async function syncFullscreenState() {
  try {
    storyFullscreen.value = await storyWindow.isFullscreen()
  } catch {
    storyFullscreen.value = false
  }
}

async function toggleStoryFullscreen() {
  const nextFullscreen = !storyFullscreen.value
  await storyWindow.setFullscreen(nextFullscreen)
  storyFullscreen.value = nextFullscreen
  await nextTick()
  scrollStoryToBottom()
}

async function openDrawer() {
  await invoke('show_drawer')
}

function toggleCompanion(id: string) {
  const ids = storyDraft.value.companionIds
  storyDraft.value.companionIds = ids.includes(id) ? ids.filter((item) => item !== id) : [...ids, id]
}

function clearCompanions() {
  storyDraft.value.companionIds = []
}

function selectAllCompanions() {
  storyDraft.value.companionIds = companions.value.map((companion) => companion.id)
}

function buildStoryCreateDraft(): StoryCreateDraft {
  const temporaryCharacters =
    storyDraft.value.temporaryName.trim() ||
    storyDraft.value.temporaryIdentity.trim() ||
    storyDraft.value.temporaryPersonality.trim()
      ? [
          {
            name: storyDraft.value.temporaryName.trim(),
            gender: '',
            ageStage: '',
            identity: storyDraft.value.temporaryIdentity.trim(),
            appearance: '',
            personality: storyDraft.value.temporaryPersonality.trim(),
            relationshipToUser: storyDraft.value.temporaryRelationship.trim(),
            relationshipToOthers: '',
            roleInStory: storyDraft.value.temporaryRole.trim(),
            speakingStyle: '',
            hiddenSetting: storyDraft.value.temporaryHidden.trim(),
            isInteractable: storyDraft.value.temporaryInteractable,
          },
        ]
      : []

  return {
    mode: storyDraft.value.mode,
    storyType: storyDraft.value.storyType.trim(),
    tone: storyDraft.value.tone.trim(),
    premise: storyDraft.value.premise.trim(),
    companionIds: [...storyDraft.value.companionIds],
    companionRole: storyDraft.value.companionRole.trim(),
    temporaryCharacters,
  }
}

async function createStory() {
  if (storyDraft.value.mode === 'custom' && !storyDraft.value.premise.trim()) {
    errorMessage.value = '自定义设定模式需要先填写故事设定。'
    return
  }

  storyLoading.value = true
  errorMessage.value = ''
  try {
    const result = await invoke<StoryTurnReply>('create_story', { draft: buildStoryCreateDraft() })
    currentStory.value = result.story
    await loadStorySaves()
    await nextTick()
    scrollStoryToBottom()
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    storyLoading.value = false
  }
}

function continueStory(story: StorySave) {
  currentStory.value = story
  errorMessage.value = ''
  void nextTick(scrollStoryToBottom)
}

async function renameStory(story: StorySave) {
  const title = window.prompt('请输入新的故事标题', story.title)?.trim()
  if (!title) {
    return
  }

  storyLoading.value = true
  try {
    const updated = await invoke<StorySave>('rename_story_save', { storyId: story.id, title })
    await loadStorySaves()
    if (currentStory.value?.id === updated.id) {
      currentStory.value = updated
    }
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    storyLoading.value = false
  }
}

async function deleteStory(story: StorySave) {
  if (!window.confirm(`确定删除故事存档「${story.title}」吗？`)) {
    return
  }

  storyLoading.value = true
  try {
    await invoke('delete_story_save', { storyId: story.id })
    if (currentStory.value?.id === story.id) {
      currentStory.value = null
    }
    await loadStorySaves()
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    storyLoading.value = false
  }
}

async function sendStoryAction() {
  const story = currentStory.value
  const content = storyInput.value.trim()
  if (!story || !content || sending.value) {
    return
  }

  storyInput.value = ''
  sending.value = true
  errorMessage.value = ''
  try {
    const result = await invoke<StoryTurnReply>('advance_story', {
      storyId: story.id,
      userInput: content,
    })
    currentStory.value = result.story
    await loadStorySaves()
    await nextTick()
    scrollStoryToBottom()
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    sending.value = false
  }
}

function chooseStoryOption(option: string) {
  if (sending.value || !currentStory.value) {
    return
  }
  storyInput.value = option
  void sendStoryAction()
}

function handleStoryInputKeydown(event: KeyboardEvent) {
  if (event.key !== 'Enter' || event.shiftKey) {
    return
  }

  event.preventDefault()
  void sendStoryAction()
}

function scrollStoryToBottom() {
  const element = storyLogRef.value
  if (element) {
    element.scrollTop = element.scrollHeight
  }
}

function providerLabel(provider?: string) {
  const labels: Record<string, string> = {
    openai: 'OpenAI 兼容',
    deepseek: 'DeepSeek',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    ollama: 'Ollama 本地',
    custom: '自定义',
  }

  return labels[(provider ?? '').toLowerCase()] ?? '自定义'
}

function companionCardSummary(companion: Companion) {
  return (
    companion.personality?.trim() ||
    companion.personaPrompt?.trim() ||
    companion.scenario?.trim() ||
    '已保存的角色卡设定'
  )
}

function extractStoryOptions(content: string) {
  const options: string[] = []
  const lines = content.split(/\r?\n/)
  for (const line of lines) {
    const match = line.trim().match(/^(?:[-*]\s*)?([1-4])[.、)]\s*(.+)$/)
    if (!match) {
      continue
    }
    const option = match[2].trim()
    if (!option || option.includes('自由输入')) {
      continue
    }
    options.push(option)
  }
  return options.slice(0, 3)
}

function storyTimeText(timestamp: number) {
  if (!timestamp) {
    return '未知时间'
  }
  return new Date(timestamp * 1000).toLocaleString()
}
</script>

<template>
  <main class="story-window" :class="[`theme-${drawerTheme}`, windowOpenAnimationClass, { 'story-window-fullscreen': storyFullscreen }]">
    <header class="story-header" @pointerdown="startDrag">
      <div>
        <h1>故事模式</h1>
        <p>{{ aiSummary }}</p>
      </div>
      <div class="story-header-actions" @pointerdown.stop>
        <button type="button" :title="storyFullscreen ? '退出全屏' : '全屏显示故事模式'" @click="toggleStoryFullscreen">
          {{ storyFullscreen ? '退出全屏' : '全屏' }}
        </button>
        <button type="button" @click="openDrawer">抽屉设置</button>
        <button type="button" title="关闭故事模式" @click="hideStory">×</button>
      </div>
    </header>

    <section v-if="!aiEnabled" class="story-notice">
      <strong>需要先配置 AI 接口</strong>
      <span>故事模式独立使用 AI 配置。打开抽屉设置，在“AI 接口”中启用并填写服务商、模型和 API Key。</span>
      <div>
        <button type="button" @click="openDrawer">打开抽屉</button>
        <button type="button" @click="loadConfig">刷新配置</button>
      </div>
    </section>

    <p v-if="errorMessage" class="story-error">{{ errorMessage }}</p>

    <section class="story-layout">
      <aside class="story-panel story-create-panel">
        <header>
          <h2>新故事</h2>
          <p>可独立生成剧情，也可以引用已有角色卡。</p>
        </header>

        <div class="story-mode-row" role="radiogroup" aria-label="故事创建方式">
          <button type="button" :class="{ active: storyDraft.mode === 'random' }" @click="storyDraft.mode = 'random'">
            随机
          </button>
          <button type="button" :class="{ active: storyDraft.mode === 'custom' }" @click="storyDraft.mode = 'custom'">
            自定义
          </button>
        </div>

        <label>
          故事类型
          <input v-model="storyDraft.storyType" placeholder="随机、校园生活、悬疑推理..." />
        </label>
        <label>
          故事氛围
          <input v-model="storyDraft.tone" placeholder="治愈、暧昧、热血、悬疑、慢热..." />
        </label>
        <label>
          故事设定
          <textarea
            v-model="storyDraft.premise"
            rows="4"
            :placeholder="
              storyDraft.mode === 'custom'
                ? '描述世界观、主角身份、关系设定、开局场景和禁止内容'
                : '随机模式可留空；填写后会作为偏好参考'
            "
          />
        </label>

        <section class="story-role-card-section">
          <div>
            <h3>引用角色卡</h3>
            <p>{{ selectedCompanionNames.length > 0 ? selectedCompanionNames.join('、') : '不引用角色卡' }}</p>
          </div>
          <div class="story-role-card-actions">
            <button type="button" :disabled="companions.length === 0" @click="selectAllCompanions">全选</button>
            <button type="button" :disabled="storyDraft.companionIds.length === 0" @click="clearCompanions">
              清空
            </button>
          </div>
          <div class="story-role-card-list">
            <label v-for="companion in companions" :key="companion.id" class="story-role-card-item">
              <input
                type="checkbox"
                :checked="storyDraft.companionIds.includes(companion.id)"
                @change="toggleCompanion(companion.id)"
              />
              <span>
                <strong>{{ companion.name }}</strong>
                <small>{{ companionCardSummary(companion) }}</small>
              </span>
            </label>
            <p v-if="companions.length === 0" class="story-empty">暂无角色卡，可先在抽屉设置中创建或导入。</p>
          </div>
        </section>

        <label>
          角色卡在故事中的定位
          <input v-model="storyDraft.companionRole" placeholder="女主、搭档、导师、对手..." />
        </label>

        <section class="story-temporary-character">
          <h3>临时角色</h3>
          <label>
            角色名称
            <input v-model="storyDraft.temporaryName" placeholder="例如：林清雪" />
          </label>
          <label>
            角色身份
            <input v-model="storyDraft.temporaryIdentity" placeholder="学生会副会长、神秘目击者..." />
          </label>
          <label>
            性格特点
            <input v-model="storyDraft.temporaryPersonality" placeholder="高冷、理性、嘴硬心软..." />
          </label>
          <label>
            与用户的关系
            <input v-model="storyDraft.temporaryRelationship" placeholder="普通同学、同伴、竞争者..." />
          </label>
          <label>
            剧情定位
            <input v-model="storyDraft.temporaryRole" placeholder="主要女主、关键线索角色..." />
          </label>
          <label>
            隐藏设定
            <textarea v-model="storyDraft.temporaryHidden" rows="2" placeholder="不会直接展示给用户，只供剧情参考" />
          </label>
          <label class="story-check-row">
            <span>可深入互动</span>
            <input v-model="storyDraft.temporaryInteractable" type="checkbox" />
          </label>
        </section>

        <button class="story-primary-button" type="button" :disabled="storyLoading || !aiEnabled" @click="createStory">
          {{ storyLoading ? '生成中...' : '生成故事开局' }}
        </button>
      </aside>

      <section class="story-panel story-play-panel">
        <header class="story-current-header">
          <div>
            <h2>{{ currentStory?.title || '未载入故事' }}</h2>
            <p v-if="currentStory">
              {{ currentStory.storyType }} / 第 {{ currentStory.currentChapter }} 章 / {{ currentStory.currentScene }}
            </p>
            <p v-else>创建新故事或从右侧存档继续。</p>
          </div>
          <div v-if="activeCharacterNames.length > 0" class="story-character-chips">
            <span v-for="name in activeCharacterNames" :key="name">{{ name }}</span>
          </div>
        </header>

        <div ref="storyLogRef" class="story-log">
          <p v-if="!currentStory" class="story-empty">故事模式不会读取普通对话记录，只会使用你选择的角色卡设定。</p>
          <article
            v-for="(message, index) in storyMessages"
            :key="`${message.timestamp}-${index}`"
            class="story-message"
            :class="message.role === 'user' ? 'user' : 'assistant'"
          >
            <span>{{ message.role === 'user' ? '你' : '故事引擎' }}</span>
            <p>{{ message.content }}</p>
            <time>{{ storyTimeText(message.timestamp) }}</time>
          </article>
          <article v-if="sending" class="story-message assistant">
            <span>故事引擎</span>
            <p>正在生成下一段剧情...</p>
          </article>
        </div>

        <section v-if="storyOptionActions.length > 0" class="story-options">
          <button
            v-for="option in storyOptionActions"
            :key="option"
            type="button"
            :disabled="sending"
            @click="chooseStoryOption(option)"
          >
            {{ option }}
          </button>
        </section>

        <form class="story-input" @submit.prevent="sendStoryAction">
          <textarea
            v-model="storyInput"
            :disabled="sending || !aiEnabled || !currentStory"
            rows="3"
            placeholder="输入你的行动、台词或选择编号，Enter 推进，Shift+Enter 换行"
            @keydown="handleStoryInputKeydown"
          />
          <button type="submit" :disabled="sending || !aiEnabled || !currentStory || storyInput.trim().length === 0">
            推进
          </button>
        </form>
      </section>

      <aside class="story-panel story-saves-panel">
        <header>
          <h2>故事存档</h2>
          <button type="button" :disabled="storyLoading" @click="loadStorySaves">刷新</button>
        </header>
        <p v-if="storySaves.length === 0" class="story-empty">暂无故事存档。</p>
        <article v-for="story in storySaves" :key="story.id" class="story-save-row">
          <div>
            <strong>{{ story.title }}</strong>
            <small>{{ story.storyType }} / 第 {{ story.currentChapter }} 章</small>
            <small>{{ storyTimeText(story.updatedAt) }}</small>
          </div>
          <p>{{ story.currentScene || story.storySummary }}</p>
          <footer>
            <button type="button" :disabled="storyLoading" @click="continueStory(story)">继续</button>
            <button type="button" :disabled="storyLoading" @click="renameStory(story)">重命名</button>
            <button class="danger-button" type="button" :disabled="storyLoading" @click="deleteStory(story)">
              删除
            </button>
          </footer>
        </article>
      </aside>
    </section>
  </main>
</template>
