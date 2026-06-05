<script setup lang="ts">
import type { AppItemKind, AppItemKindFilter } from '../types/app'

const props = defineProps<{
  modelValue: string
  quickTags: string[]
  activeKinds: AppItemKind[]
  kindOptions: { value: AppItemKindFilter; label: string }[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'update:activeKinds': [value: AppItemKind[]]
  add: []
  quickTag: [tag: string]
}>()

function kindButtonActive(value: AppItemKindFilter) {
  return value === 'all' ? props.activeKinds.length === 0 : props.activeKinds.includes(value)
}

function toggleKind(value: AppItemKindFilter) {
  if (value === 'all') {
    emit('update:activeKinds', [])
    return
  }

  const next = props.activeKinds.includes(value)
    ? props.activeKinds.filter((item) => item !== value)
    : [...props.activeKinds, value]
  emit('update:activeKinds', next)
}
</script>

<template>
  <div class="search-row">
    <div class="search-main">
      <input
        class="search-input"
        :value="modelValue"
        placeholder="搜索名称、分类、标签、路径或网址"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      />
      <div class="search-quick-tags" v-if="quickTags.length > 0">
        <button
          v-for="tag in quickTags"
          :key="tag"
          type="button"
          :title="`搜索 ${tag}`"
          @click="emit('quickTag', tag)"
        >
          {{ tag }}
        </button>
      </div>
    </div>
    <div class="kind-filter" aria-label="快捷类型">
      <button
        v-for="option in kindOptions"
        :key="option.value"
        type="button"
        :class="{ active: kindButtonActive(option.value) }"
        :aria-pressed="kindButtonActive(option.value)"
        @click="toggleKind(option.value)"
      >
        {{ option.label }}
      </button>
    </div>
    <div class="add-actions">
      <button class="primary-button" type="button" @click="emit('add')">添加</button>
    </div>
  </div>
</template>
