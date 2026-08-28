import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    environmentOptions: {
      jsdom: {
        pretendToBeVisual: true,
      },
    },
    include: ['src/**/*.spec.ts'],
    setupFiles: ['./src/tests/setup.ts'],
    clearMocks: true,
    restoreMocks: true,
  },
})
