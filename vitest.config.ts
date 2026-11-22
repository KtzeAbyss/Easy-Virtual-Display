import { resolve } from 'path'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [tailwindcss(), react()],
  define: { __APP_VERSION__: JSON.stringify('0.0.0-test') },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/renderer/__tests__/setup.ts'],
    globals: true,
    include: [
      'src/renderer/__tests__/**/*.{test,spec}.{ts,tsx}',
      'src/shared/locales/__tests__/**/*.{test,spec}.{ts,tsx}'
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      include: [
        'src/renderer/hooks/**/*.{ts,tsx}',
        'src/renderer/bridge/**/*.{ts,tsx}',
        'src/renderer/lib/**/*.{ts,tsx}',
        'src/renderer/components/*Page.tsx',
        'src/renderer/components/driver/**/*.{ts,tsx}',
        'src/shared/errors.ts'
      ]
    }
  },
  resolve: {
    alias: {
      '@': resolve('src/renderer')
    }
  }
})
