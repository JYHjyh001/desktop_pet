<script setup lang="ts">
import type { AppItemKindFilter } from '../types/app'

defineProps<{
  modelValue: string
  quickTags: string[]
  activeKind: AppItemKindFilter
  kindOptions: { value: AppItemKindFilter; label: string }[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'update:activeKind': [value: AppItemKindFilter]
  add: []
  quickTag: [tag: string]
}>()
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
        :class="{ active: activeKind === option.value }"
        @click="emit('update:activeKind', option.value)"
      >
        {{ option.label }}
      </button>
    </div>
    <div class="add-actions">
      <button class="primary-button" type="button" @click="emit('add')">添加</button>
    </div>
  </div>
</template>
