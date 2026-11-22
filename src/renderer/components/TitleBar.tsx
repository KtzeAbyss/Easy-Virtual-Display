import { useTranslation } from 'react-i18next'
import appIcon from '../../../resources/icon.png'
import { TauriWindowControls } from './TauriWindowControls'

export function TitleBar(): React.JSX.Element {
  const { t } = useTranslation()
  return (
    // `deep` lets the whole subtree initiate a drag — bare/`true` only fires when the
    // mousedown lands on this element itself, which would leave the icon, title text and
    // spacer un-draggable. The window controls opt out via `data-tauri-drag-region="false"`.
    <header className="flex h-8 shrink-0 items-center" data-tauri-drag-region="deep">
      <div className="flex min-w-0 flex-1 items-center gap-3 pl-4">
        <img src={appIcon} alt="" className="size-4" draggable={false} />
        <span className="truncate text-xs">{t('common:app_title')}</span>
      </div>
      <TauriWindowControls />
    </header>
  )
}
