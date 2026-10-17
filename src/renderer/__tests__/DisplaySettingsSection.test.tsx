import { describe, expect, it } from 'vitest'
import { renderWithI18n, screen } from './test-utils'
import { DisplaySettingsSection } from '../components/settings-panel/DisplaySettingsSection'
import { DEFAULT_APP_SETTINGS } from '../../shared'

describe('DisplaySettingsSection', () => {
  it('shows nothing extra while the fallback display works', () => {
    renderWithI18n(
      <DisplaySettingsSection
        settings={{ ...DEFAULT_APP_SETTINGS, fallbackDisplay: true }}
        fallbackError={null}
        onUpdate={() => {}}
        disabled={false}
      />
    )

    expect(screen.queryByText(/could not add the fallback display/i)).not.toBeInTheDocument()
  })

  it('explains why the fallback display could not be added', () => {
    renderWithI18n(
      <DisplaySettingsSection
        settings={{ ...DEFAULT_APP_SETTINGS, fallbackDisplay: true }}
        fallbackError="limit_exceeded"
        onUpdate={() => {}}
        disabled={false}
      />
    )

    expect(
      screen.getByText('Could not add the fallback display: Display limit exceeded')
    ).toBeInTheDocument()
  })
})
