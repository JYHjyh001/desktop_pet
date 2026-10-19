<script setup lang="ts">
import { computed, ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { safeChatImageSource } from '../utils/chatImages'

const props = defineProps<{ src: string; alt: string }>()
const emit = defineEmits<{ loaded: [] }>()
const source = computed(() => safeChatImageSource(props.src))
const failed = ref(false)
const loading = ref(true)
const expanded = ref(false)
const resolved = ref('')
const figure = ref<HTMLElement | null>(null)
const visible = ref(false)
let observer: IntersectionObserver | null = null
let version = 0
let disposed = false
let intersecting = false
function visibilityChanged() {
  visible.value = intersecting && document.visibilityState !== 'hidden'
  if (visible.value) void resolveImage()
  else { version += 1; resolved.value = ''; loading.value = true }
}
async function resolveImage() {
  const current = ++version
  if (!source.value || !visible.value) return
  failed.value = false
  loading.value = true
  try {
    const result = await invoke<string>('load_chat_image', { source: source.value })
    if (!disposed && current === version && visible.value) resolved.value = result
  } catch {
    if (!disposed && current === version) { failed.value = true; loading.value = false }
  }
}
onMounted(() => {
  document.addEventListener('visibilitychange', visibilityChanged)
  if (typeof IntersectionObserver === 'undefined') { intersecting = true; visibilityChanged(); return }
  observer = new IntersectionObserver(([entry]) => {
    intersecting = entry.isIntersecting
    visibilityChanged()
  }, { rootMargin: '200px' })
  if (figure.value) observer.observe(figure.value)
})
onBeforeUnmount(() => { document.removeEventListener('visibilitychange', visibilityChanged); disposed = true; version += 1; observer?.disconnect(); resolved.value = '' })
watch(() => props.src, () => { failed.value = false; loading.value = true; expanded.value = false; resolved.value = ''; void resolveImage() })
function retry() {
  failed.value = false
  loading.value = true
  void resolveImage()
}
function loaded() {
  loading.value = false
  emit('loaded')
}
</script>

<template>
  <figure ref="figure" class="chat-reply-image" :class="{ expanded }">
    <template v-if="source && !failed">
      <button
        type="button"
        class="chat-reply-image-toggle"
        :aria-label="expanded ? '收起图片' : '展开图片'"
        :aria-expanded="expanded"
        @click.stop="expanded = !expanded"
      >
        <img
          v-if="resolved"
          :src="resolved"
          :alt="alt"
          referrerpolicy="no-referrer"
          loading="lazy"
          decoding="async"
          draggable="false"
          @load="loaded"
          @error="failed = true; loading = false"
        />
      </button>
      <figcaption aria-live="polite">{{ loading ? '图片加载中…' : expanded ? '点击收起图片' : '点击展开图片' }}</figcaption>
    </template>
    <div v-else class="chat-reply-image-error" role="status">
      <span>{{ source ? '图片加载失败，链接可能已失效或暂时无法访问。' : '图片地址或格式不支持，或图片超过大小限制。' }}</span>
      <button v-if="source" type="button" @click.stop="retry">重试</button>
    </div>
  </figure>
</template>

<style>
.chat-reply-image {
  --image-panel: #f0f4fa;
  --image-border: #cbd5e1;
  --image-text: #475569;
  --image-button: #e0e9f5;
  margin: 6px 0;
  width: 100%;
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--image-border);
  border-radius: 12px;
  background: var(--image-panel);
  color: var(--image-text);
}
.theme-animal-island .chat-reply-image {
  --image-panel: #fff9e9;
  --image-border: #c9c39a;
  --image-text: #52624a;
  --image-button: #e2ebd0;
}
.chat-reply-image-toggle {
  display: block;
  width: 100%;
  padding: 4px;
  background: transparent;
  cursor: zoom-in;
}
.chat-reply-image img { display: block; width: 100%; max-height: 220px; min-height: 48px; object-fit: contain; border-radius: 8px; }
.chat-reply-image.expanded img { max-height: none; }
.chat-reply-image.expanded .chat-reply-image-toggle { cursor: zoom-out; }
.chat-reply-image-toggle { min-height: 48px; }
.chat-reply-image figcaption { padding: 5px 10px 8px; font-size: 11px; text-align: center; }
.chat-reply-image-error { display: grid; gap: 8px; padding: 12px; font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }
.chat-reply-image-error button { justify-self: start; border-radius: 7px; padding: 5px 12px; background: var(--image-button); color: inherit; }
.chat-reply-image button:focus-visible { outline: 2px solid var(--image-text); outline-offset: -3px; }
</style>
