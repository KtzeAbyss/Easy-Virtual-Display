import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { UseMutationResult } from '@tanstack/react-query'
import { Monitor, MoreHorizontal, Plus, Settings2, Trash2 } from 'lucide-react'
import type { AppSnapshot } from '../../shared'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { DisplayCard } from './DisplayCard'
import { PageHeader } from './layout/Page'
import { DriverStatusInfoBar, InstallDriverButton } from './driver/DriverStatus'
import { Button } from './ui/button'
import { ConfirmDialog } from './ui/dialog'
import { Tooltip } from './ui/tooltip'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from './ui/dropdown-menu'

export type AddDisplayMutation = UseMutationResult<void, unknown, void>

/** Add is blocked when the driver is unhealthy or every slot is already taken. */
export function canAddDisplay(snapshot: AppSnapshot): boolean {
  const { host } = snapshot
  return host.status === 'ok' && host.displays.length < host.maxDisplays
}

interface DisplaysPageProps {
  snapshot: AppSnapshot
  addDisplay: AddDisplayMutation
  onOpenDriverPage: () => void
}

function AddDisplayButton({
  snapshot,
  addDisplay
}: {
  snapshot: AppSnapshot
  addDisplay: AddDisplayMutation
}): React.JSX.Element {
  const { t } = useTranslation()
  const { host } = snapshot
  const atLimit = host.maxDisplays > 0 && host.displays.length >= host.maxDisplays
  const button = (
    <Button
      onClick={() => addDisplay.mutate()}
      disabled={!canAddDisplay(snapshot) || addDisplay.isPending}
    >
      <Plus />
      {t('displays:add_display')}
    </Button>
  )
  if (!atLimit) return button
  // A disabled button swallows pointer events, so the tooltip hangs off a focusable wrapper.
  return (
    <Tooltip content={t('displays:limit_reached', { max: host.maxDisplays })}>
      <span tabIndex={0} className="rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring">
        {button}
      </span>
    </Tooltip>
  )
}

function PageMenu({
  canRemoveAll,
  onRemoveAll
}: {
  canRemoveAll: boolean
  onRemoveAll: () => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" aria-label={t('common:more_actions')}>
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onSelect={() => void window.easyVirtualDisplay.openDisplaySettings()}>
          <Settings2 />
          {t('displays:open_display_settings')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem destructive disabled={!canRemoveAll} onSelect={onRemoveAll}>
          <Trash2 />
          {t('displays:remove_all')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function EmptyState({
  snapshot,
  addDisplay
}: {
  snapshot: AppSnapshot
  addDisplay: AddDisplayMutation
}): React.JSX.Element {
  const { t } = useTranslation()
  const needsDriver = snapshot.host.status === 'not_installed'
  return (
    <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-6 py-14 text-center">
      <Monitor className="mb-2 size-10 text-muted-foreground" strokeWidth={1.25} aria-hidden />
      <p className="font-display text-base font-semibold">
        {t(needsDriver ? 'displays:empty_no_driver_title' : 'displays:empty_title')}
      </p>
      <p className="max-w-sm text-sm text-muted-foreground">
        {t(needsDriver ? 'displays:empty_no_driver_description' : 'displays:empty_description')}
      </p>
      <div className="mt-3">
        {needsDriver ? (
          <InstallDriverButton />
        ) : (
          <AddDisplayButton snapshot={snapshot} addDisplay={addDisplay} />
        )}
      </div>
    </div>
  )
}

export function DisplaysPage({
  snapshot,
  addDisplay,
  onOpenDriverPage
}: DisplaysPageProps): React.JSX.Element {
  const { t } = useTranslation()
  const { host } = snapshot
  const driverOk = host.status === 'ok'
  const displays = host.displays
  const [confirmRemoveAll, setConfirmRemoveAll] = useState(false)

  const removeAll = useBridgeCommand<void>(() => window.easyVirtualDisplay.removeAllDisplays())

  const meta =
    host.maxDisplays > 0 ? (
      <span className="text-sm text-muted-foreground tabular-nums">
        {t('displays:count_of_max', { count: displays.length, max: host.maxDisplays })}
      </span>
    ) : null

  return (
    <>
      <PageHeader
        title={t('displays:title')}
        meta={meta}
        actions={
          <>
            <AddDisplayButton snapshot={snapshot} addDisplay={addDisplay} />
            <PageMenu
              canRemoveAll={driverOk && displays.length > 0 && !removeAll.isPending}
              onRemoveAll={() => setConfirmRemoveAll(true)}
            />
          </>
        }
      />
      <div className="flex flex-col gap-2">
        {host.status !== 'not_installed' && (
          <DriverStatusInfoBar status={host.status} onOpenDriverPage={onOpenDriverPage} />
        )}
        {displays.length === 0 ? (
          <EmptyState snapshot={snapshot} addDisplay={addDisplay} />
        ) : (
          displays.map((display) => (
            <DisplayCard
              key={display.identifier}
              display={display}
              driverOk={driverOk}
              viewerOpen={snapshot.openViewers.includes(display.index)}
            />
          ))
        )}
      </div>
      <ConfirmDialog
        open={confirmRemoveAll}
        onOpenChange={setConfirmRemoveAll}
        title={t('displays:remove_all_confirm_title')}
        description={t('displays:remove_all_confirm_body', { count: displays.length })}
        confirmLabel={t('displays:remove_all')}
        cancelLabel={t('common:cancel')}
        pending={removeAll.isPending}
        onConfirm={() => removeAll.mutate(undefined, { onSettled: () => setConfirmRemoveAll(false) })}
      />
    </>
  )
}
