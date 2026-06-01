import './assets/main.css'

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { Toaster } from 'sonner'
import App from './App'
import { createTauriBridge } from './bridge/tauri-bridge'
import { initRendererI18n } from './i18n'

const queryClient = new QueryClient()

// Install the shell bridge before i18n boots, so `window.easyVirtualDisplay.getSnapshot()`
// resolves correctly during init. (Tests stub `window.easyVirtualDisplay` directly in
// __tests__/setup.ts and never import main.tsx, so this unconditional install is safe.)
window.easyVirtualDisplay = createTauriBridge()

initRendererI18n().then((i18n) => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <App />
          <Toaster position="bottom-right" richColors />
        </QueryClientProvider>
      </I18nextProvider>
    </StrictMode>
  )
})
