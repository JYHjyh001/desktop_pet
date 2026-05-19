<script setup lang="ts">
defineProps<{
  modelValue: string
  quickTags: string[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  add: []
  quickTag: [tag: string]
}>()
</script>

<template>
  <div class="search-row">
    <input
      class="search-input"
      :value="modelValue"
      placeholder="搜索软件、分类或标签"
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
    <button class="primary-button" type="button" @click="emit('add')">添加软件</button>
  </div>
</template>
