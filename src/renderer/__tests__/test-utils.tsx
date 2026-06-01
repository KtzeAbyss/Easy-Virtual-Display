import { render, screen, waitFor, act, type RenderResult } from '@testing-library/react'
import type { RenderOptions } from '@testing-library/react'
import type { ReactElement } from 'react'
import i18next from 'i18next'
import { I18nextProvider } from 'react-i18next'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { resources } from '../../shared/locales'

export function renderWithI18n(ui: ReactElement, options?: RenderOptions): RenderResult {
  const i18n = i18next.createInstance({
    resources,
    fallbackLng: 'en',
    initImmediate: false
  })
  i18n.init()

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } }
  })

  return render(ui, {
    wrapper: ({ children }) => (
      <QueryClientProvider client={queryClient}>
        <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
      </QueryClientProvider>
    ),
    ...options
  })
}

export { screen, waitFor, act }
