import { useTranslation } from 'react-i18next'
import { ExternalLink } from 'lucide-react'
import type { ProjectLink } from '../../shared'
import appIcon from '../../../resources/icon.png'
import { PageHeader, Section, SettingRow } from './layout/Page'
import { Button } from './ui/button'

const LINKS: ProjectLink[] = ['repository', 'issues', 'releases']

const SHORTCUTS: { keys: string; labelKey: string }[] = [
  { keys: 'Ctrl+N', labelKey: 'common:about.shortcut_add_display' },
  { keys: 'Ctrl+,', labelKey: 'common:about.shortcut_settings' }
]

export function AboutPage(): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <>
      <PageHeader title={t('common:nav.about')} />
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-4 rounded-md border bg-card px-4 py-4">
          <img src={appIcon} alt="" className="size-12" draggable={false} />
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="font-display text-base font-semibold">{t('common:app_title')}</span>
            <span className="selectable text-xs text-muted-foreground tabular-nums">
              {t('common:about.version', { version: __APP_VERSION__ })}
            </span>
            <span className="text-xs text-muted-foreground">{t('common:about.description')}</span>
          </div>
        </div>
        <p className="px-0.5 pt-1 text-xs text-muted-foreground">{t('common:about.driver_credit')}</p>

        <Section title={t('common:about.links_title')}>
          <div className="flex flex-wrap gap-2">
            {LINKS.map((link) => (
              <Button
                key={link}
                variant="outline"
                onClick={() => void window.easyVirtualDisplay.openProjectLink(link)}
              >
                {t(`common:about.${link}`)}
                <ExternalLink className="size-3.5 text-muted-foreground" />
              </Button>
            ))}
          </div>
        </Section>

        <Section title={t('common:about.shortcuts_title')}>
          {SHORTCUTS.map(({ keys, labelKey }) => (
            <SettingRow key={keys} label={t(labelKey)}>
              <kbd className="rounded-md border bg-control px-2 py-0.5 font-sans text-xs">{keys}</kbd>
            </SettingRow>
          ))}
        </Section>
      </div>
    </>
  )
}
