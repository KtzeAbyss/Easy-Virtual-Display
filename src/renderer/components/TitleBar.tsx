import { useTranslation } from 'react-i18next'
import type { AppTheme } from '../../shared'
import { cn } from '@/lib/utils'
import { Monitor, Settings, Sun, Moon, Laptop } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { TauriWindowControls } from './TauriWindowControls'

const THEME_OPTIONS: { value: AppTheme; icon: LucideIcon }[] = [
  { value: 'system', icon: Laptop },
  { value: 'light', icon: Sun },
  { value: 'dark', icon: Moon }
]

const TITLE_TABS: { value: 'displays' | 'settings'; icon: LucideIcon; labelKey: string }[] = [
  { value: 'displays', icon: Monitor, labelKey: 'common:tab_displays' },
  { value: 'settings', icon: Settings, labelKey: 'common:tab_settings' }
]

function ThemeSwitcher({
  theme,
  onThemeChange
}: {
  theme: AppTheme
  onThemeChange: (t: AppTheme) => void
}): React.JSX.Element {
  return (
    <div className="flex items-center gap-0.5 bg-muted p-0.5 rounded-md">
      {THEME_OPTIONS.map(({ value, icon: Icon }) => (
        <button
          key={value}
          onClick={() => onThemeChange(value)}
          className={cn(
            'flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-all',
            theme === value && 'bg-card text-foreground shadow-sm'
          )}
          data-tauri-drag-region="false"
          aria-label={value}
        >
          <Icon className="w-3 h-3" />
        </button>
      ))}
    </div>
  )
}

function TitleTabs({
  activeTab,
  onTabChange
}: {
  activeTab: string
  onTabChange: (tab: 'displays' | 'settings') => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex gap-0 px-4 pb-0">
      {TITLE_TABS.map(({ value, icon: Icon, labelKey }) => (
        <button
          key={value}
          onClick={() => onTabChange(value)}
          className={cn(
            'flex items-center gap-1.5 px-3 py-2 text-sm border-b-2 transition-colors',
            activeTab === value
              ? 'border-primary text-foreground font-medium'
              : 'border-transparent text-muted-foreground hover:text-foreground'
          )}
        >
          <Icon className="w-3.5 h-3.5" />
          {t(labelKey)}
        </button>
      ))}
    </div>
  )
}

interface TitleBarProps {
  activeTab: 'displays' | 'settings'
  onTabChange: (tab: 'displays' | 'settings') => void
  theme: AppTheme
  onThemeChange: (theme: AppTheme) => void
}

export function TitleBar({
  activeTab,
  onTabChange,
  theme,
  onThemeChange
}: TitleBarProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="sticky top-0 z-10 bg-sidebar border-b border-sidebar-border">
      {/* `deep` lets the whole subtree initiate a drag — bare/`true` only fires when the
          mousedown lands on this element itself, which would leave the logo, title text and
          spacer un-draggable. Interactive children (theme buttons, window controls) opt out
          explicitly via `data-tauri-drag-region="false"`. */}
      <div
        className="flex items-center gap-3 pl-4 pr-0 py-3"
        data-tauri-drag-region="deep"
      >
        <div className="flex items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded bg-primary/15">
            <Monitor className="w-3.5 h-3.5 text-primary" />
          </div>
          <span className="text-sm font-semibold text-foreground">{t('common:app_title')}</span>
        </div>
        <div className="flex-1" />
        <ThemeSwitcher theme={theme} onThemeChange={onThemeChange} />
        <TauriWindowControls />
      </div>
      <TitleTabs activeTab={activeTab} onTabChange={onTabChange} />
    </div>
  )
}
