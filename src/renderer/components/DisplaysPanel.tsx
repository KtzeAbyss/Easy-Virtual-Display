import { useTranslation } from 'react-i18next'
import type { AppSnapshot, DisplaySummary } from '../../shared'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { DisplayCard } from './DisplayCard'
import { Button } from './ui/button'
import { MonitorOff, Plus, Trash2 } from 'lucide-react'
import type { UseMutationResult } from '@tanstack/react-query'

interface DisplaysPanelProps {
  snapshot: AppSnapshot
}

interface PanelHeaderProps {
  hasDisplays: boolean
  activeCount: number
  allDisabled: boolean
  addDisplay: UseMutationResult<void, unknown, void>
  removeAll: UseMutationResult<void, unknown, void>
}

function DisplaysPanelHeader({
  hasDisplays,
  activeCount,
  allDisabled,
  addDisplay,
  removeAll
}: PanelHeaderProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-2">
        <h2 className="text-sm font-semibold text-foreground">{t('displays:title')}</h2>
        {hasDisplays && (
          <span className="text-xs font-mono text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
            {t('displays:active_badge', { count: activeCount })}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2">
        {hasDisplays && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs gap-1.5 text-muted-foreground hover:text-destructive-foreground hover:bg-destructive/10"
            onClick={() => removeAll.mutate()}
            disabled={allDisabled || removeAll.isPending}
          >
            <Trash2 className="w-3 h-3" />
            {t('displays:remove_all')}
          </Button>
        )}
        <Button
          size="sm"
          className="h-7 text-xs gap-1.5"
          onClick={() => addDisplay.mutate()}
          disabled={allDisabled || addDisplay.isPending}
        >
          <Plus className="w-3.5 h-3.5" />
          {t('displays:add_display')}
        </Button>
      </div>
    </div>
  )
}

interface EmptyStateProps {
  allDisabled: boolean
  addDisplay: UseMutationResult<void, unknown, void>
}

function DisplaysEmptyState({ allDisabled, addDisplay }: EmptyStateProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-12 bg-card border border-border border-dashed rounded-lg">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-muted">
        <MonitorOff className="w-5 h-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col items-center gap-1">
        <p className="text-sm font-medium text-foreground">{t('displays:empty_title')}</p>
        <p className="text-xs text-muted-foreground">{t('displays:empty_description')}</p>
      </div>
      <Button
        size="sm"
        className="gap-1.5 mt-1"
        onClick={() => addDisplay.mutate()}
        disabled={allDisabled || addDisplay.isPending}
      >
        <Plus className="w-3.5 h-3.5" />
        {t('displays:add_display')}
      </Button>
    </div>
  )
}

function DisplaysList({
  displays,
  driverOk
}: {
  displays: DisplaySummary[]
  driverOk: boolean
}): React.JSX.Element {
  return (
    <div className="flex flex-col gap-2">
      {displays.map((display) => (
        <DisplayCard key={display.identifier} display={display} driverOk={driverOk} />
      ))}
    </div>
  )
}

export function DisplaysPanel({ snapshot }: DisplaysPanelProps): React.JSX.Element {
  const { host } = snapshot
  const driverOk = host.status === 'ok'
  const displays = host.displays
  const hasDisplays = displays.length > 0

  const addDisplay = useBridgeCommand<void>(() => window.easyVirtualDisplay.addDisplay())
  const removeAll = useBridgeCommand<void>(() => window.easyVirtualDisplay.removeAllDisplays())
  const allDisabled = !driverOk

  return (
    <div className="flex flex-col gap-3">
      <DisplaysPanelHeader
        hasDisplays={hasDisplays}
        activeCount={displays.filter((d) => d.active).length}
        allDisabled={allDisabled}
        addDisplay={addDisplay}
        removeAll={removeAll}
      />
      {!hasDisplays && <DisplaysEmptyState allDisabled={allDisabled} addDisplay={addDisplay} />}
      {hasDisplays && <DisplaysList displays={displays} driverOk={driverOk} />}
    </div>
  )
}
