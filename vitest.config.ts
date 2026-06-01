import { resolve } from 'path'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [tailwindcss(), react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['src/renderer/__tests__/setup.ts'],
    globals: true,
    include: [
      'src/renderer/__tests__/**/*.{test,spec}.{ts,tsx}',
      'src/shared/locales/__tests__/**/*.{test,spec}.{ts,tsx}'
    ]
  },
  resolve: {
    alias: {
      '@': resolve('src/renderer')
    }
  }
})
