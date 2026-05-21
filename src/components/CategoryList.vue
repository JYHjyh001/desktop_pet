<script setup lang="ts">
defineProps<{
  categories: string[]
  active: string
}>()

const emit = defineEmits<{
  select: [category: string]
}>()
</script>

<template>
  <nav class="category-list" aria-label="快捷入口分类">
    <button
      v-for="item in categories"
      :key="item"
      class="category-button"
      :class="{ active: active === item }"
      type="button"
      @click="emit('select', item)"
    >
      {{ item }}
    </button>
  </nav>
</template>
