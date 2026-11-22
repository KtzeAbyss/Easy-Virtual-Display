import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { toast } from 'sonner'
import { AlertTriangle, Eye, EyeOff, MoreHorizontal, Settings2, Trash2 } from 'lucide-react'
import type { DisplaySummary, Orientation, SetDisplayModeInput } from '../../shared'
import { cn } from '@/lib/utils'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { Button } from './ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select'
import { Tooltip } from './ui/tooltip'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from './ui/dropdown-menu'

interface DisplayCardProps {
  display: DisplaySummary
  driverOk: boolean
  viewerOpen: boolean
}

const ORIENTATION_KEYS: Record<Orientation, string> = {
  landscape: 'displays:orientations.landscape',
  portrait: 'displays:orientations.portrait',
  landscape_flipped: 'displays:orientations.landscape_flipped',
  portrait_flipped: 'displays:orientations.portrait_flipped'
}

function modeSummary(display: DisplaySummary, t: TFunction): string | null {
  const mode = display.currentMode
  if (!mode) return null
  return `${mode.width} × ${mode.height} · ${t('displays:hz', { value: mode.hz })} · ${t(ORIENTATION_KEYS[display.currentOrientation])}`
}

function ActiveChip({ active }: { active: boolean }): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center rounded-full px-2 text-xs',
        active ? 'bg-status-ok/15 text-status-ok' : 'bg-muted text-muted-foreground'
      )}
    >
      {t(active ? 'displays:state_active' : 'displays:state_inactive')}
    </span>
  )
}

function Field({
  label,
  children
}: {
  label: string
  children: (id: string) => React.ReactNode
}): React.JSX.Element {
  const id = useId()
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        {label}
      </label>
      {children(id)}
    </div>
  )
}

function CardMenu({
  removeDisabled,
  onRemove
}: {
  removeDisabled: boolean
  onRemove: () => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t('common:more_actions')}
          className="text-muted-foreground"
        >
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onSelect={() => void window.easyVirtualDisplay.openDisplaySettings()}>
          <Settings2 />
          {t('displays:open_display_settings')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem destructive disabled={removeDisabled} onSelect={onRemove}>
          <Trash2 />
          {t('displays:remove')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function DisplayCard({ display, driverOk, viewerOpen }: DisplayCardProps): React.JSX.Element {
  const { t } = useTranslation()
  const setMode = useBridgeCommand((input: SetDisplayModeInput) =>
    window.easyVirtualDisplay.setDisplayMode(input)
  )
  const removeDisplay = useBridgeCommand((index: number) =>
    window.easyVirtualDisplay.removeDisplay(index)
  )
  const toggleViewer = useBridgeCommand((open: boolean) =>
    open
      ? window.easyVirtualDisplay.openDisplayViewer(display.index)
      : window.easyVirtualDisplay.closeDisplayViewer(display.index)
  )
  const busy = setMode.isPending || removeDisplay.isPending || toggleViewer.isPending
  const isDisabled = !driverOk || busy

  const mode = display.currentMode
  const currentResKey = mode ? `${mode.width}x${mode.height}` : ''
  const availableHz =
    display.supportedResolutions.find((r) => r.width === mode?.width && r.height === mode?.height)
      ?.refreshRates ?? []
  const currentHz = mode?.hz ?? 0
  const summary = modeSummary(display, t)

  const changeMode = (patch: Omit<SetDisplayModeInput, 'index'>): void => {
    const previous: SetDisplayModeInput = {
      index: display.index,
      ...(mode ?? {}),
      orientation: display.currentOrientation
    }
    setMode.mutate(
      { index: display.index, ...patch },
      {
        onSuccess: () => {
          toast(t('displays:mode_changed'), {
            action: { label: t('common:undo'), onClick: () => setMode.mutate(previous) }
          })
        }
      }
    )
  }

  const handleResolutionChange = (val: string): void => {
    const [w, h] = val.split('x').map(Number)
    const res = display.supportedResolutions.find((r) => r.width === w && r.height === h)
    if (!res) return
    const nextHz = res.refreshRates.includes(currentHz) ? currentHz : res.refreshRates[0]
    changeMode({ width: w, height: h, hz: nextHz })
  }

  // Opening needs an active display and a healthy driver; closing must stay available for
  // as long as a window is open, even when the driver faults.
  const viewerDisabled = viewerOpen ? toggleViewer.isPending : isDisabled || !display.active

  return (
    <article className="rounded-md border bg-card">
      <div className="flex items-center gap-2.5 px-3.5 py-2">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex min-w-0 items-center gap-2">
            <Tooltip content={t('displays:device_name', { name: display.deviceName })}>
              <h3 className="truncate text-sm font-semibold">
                {display.displayName || display.deviceName}
              </h3>
            </Tooltip>
            <ActiveChip active={display.active} />
            {display.unsupportedCurrentMode && (
              <span className="inline-flex items-center gap-1 text-xs text-status-warn">
                <AlertTriangle className="size-3.5" aria-hidden />
                {t('displays:unsupported_mode')}
              </span>
            )}
          </div>
          {summary && <p className="text-xs text-muted-foreground tabular-nums">{summary}</p>}
        </div>
        <Tooltip content={t('displays:viewer_hint')}>
          <Button
            variant="outline"
            size="sm"
            onClick={() => toggleViewer.mutate(!viewerOpen)}
            disabled={viewerDisabled}
          >
            {viewerOpen ? <EyeOff /> : <Eye />}
            {t(viewerOpen ? 'displays:close_viewer' : 'displays:open_viewer')}
          </Button>
        </Tooltip>
        <CardMenu
          removeDisabled={isDisabled}
          onRemove={() => removeDisplay.mutate(display.index)}
        />
      </div>
      <div className="grid grid-cols-3 gap-2.5 border-t px-3.5 pt-2 pb-2.5">
        <Field label={t('displays:resolution')}>
          {(id) => (
            <Select value={currentResKey} onValueChange={handleResolutionChange} disabled={isDisabled}>
              <SelectTrigger id={id} className="w-full tabular-nums">
                <SelectValue placeholder={t('displays:resolution')} />
              </SelectTrigger>
              <SelectContent>
                {display.supportedResolutions.map((r) => {
                  const key = `${r.width}x${r.height}`
                  return (
                    <SelectItem key={key} value={key} className="tabular-nums">
                      {r.width} × {r.height}
                    </SelectItem>
                  )
                })}
              </SelectContent>
            </Select>
          )}
        </Field>
        <Field label={t('displays:refresh_rate')}>
          {(id) => (
            <Select
              value={String(currentHz)}
              onValueChange={(val) => changeMode({ hz: Number(val) })}
              disabled={isDisabled || availableHz.length === 0}
            >
              <SelectTrigger id={id} className="w-full tabular-nums">
                <SelectValue placeholder={t('displays:refresh_rate')} />
              </SelectTrigger>
              <SelectContent>
                {availableHz.map((hz) => (
                  <SelectItem key={hz} value={String(hz)} className="tabular-nums">
                    {t('displays:hz', { value: hz })}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </Field>
        <Field label={t('displays:orientation')}>
          {(id) => (
            <Select
              value={display.currentOrientation}
              onValueChange={(val) => changeMode({ orientation: val as Orientation })}
              disabled={isDisabled}
            >
              <SelectTrigger id={id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(Object.entries(ORIENTATION_KEYS) as [Orientation, string][]).map(
                  ([value, key]) => (
                    <SelectItem key={value} value={value}>
                      {t(key)}
                    </SelectItem>
                  )
                )}
              </SelectContent>
            </Select>
          )}
        </Field>
      </div>
    </article>
  )
}
