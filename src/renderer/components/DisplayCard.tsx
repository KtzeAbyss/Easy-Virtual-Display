import type { DisplaySummary, Orientation } from '../../shared'
import { useTranslation } from 'react-i18next'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { Button } from './ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select'
import { cn } from '@/lib/utils'
import { AlertTriangle, Monitor, Trash2 } from 'lucide-react'

interface DisplayCardProps {
  display: DisplaySummary
  driverOk: boolean
}

const ORIENTATION_KEYS: Record<Orientation, string> = {
  landscape: 'displays:orientations.landscape',
  portrait: 'displays:orientations.portrait',
  landscape_flipped: 'displays:orientations.landscape_flipped',
  portrait_flipped: 'displays:orientations.portrait_flipped'
}

function useDisplayCardModel(display: DisplaySummary): {
  currentResKey: string
  availableHz: number[]
  currentHz: number
} {
  const currentResKey = display.currentMode
    ? `${display.currentMode.width}x${display.currentMode.height}`
    : ''
  const currentResolution = display.supportedResolutions.find(
    (r) => r.width === display.currentMode?.width && r.height === display.currentMode?.height
  )
  const availableHz = currentResolution?.refreshRates ?? []
  const currentHz = display.currentMode?.hz ?? 0
  return { currentResKey, availableHz, currentHz }
}

function getModeSummary(
  display: DisplaySummary,
  t: ReturnType<typeof useTranslation>['t']
): string | null {
  if (!display.currentMode) return null
  return `${display.currentMode.width}×${display.currentMode.height} @ ${t('displays:hz', { value: display.currentMode.hz })} · ${t(ORIENTATION_KEYS[display.currentOrientation])}`
}

function DisplayCardHeader({
  display,
  isDisabled,
  onRemove
}: {
  display: DisplaySummary
  isDisabled: boolean
  onRemove: () => void
}): React.JSX.Element {
  const { t } = useTranslation()
  const modeSummary = getModeSummary(display, t)
  return (
    <div className="flex items-start justify-between px-4 pt-4 pb-3">
      <div className="flex items-start gap-3">
        <div className="mt-0.5 flex h-8 w-8 items-center justify-center rounded-md bg-primary/10">
          <Monitor className="w-4 h-4 text-primary" />
        </div>
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-foreground">
              {display.displayName || display.deviceName}
            </span>
            <span className="text-xs font-mono text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
              #{display.index}
            </span>
            {display.unsupportedCurrentMode && (
              <div className="flex items-center gap-1 text-status-warn">
                <AlertTriangle className="w-3.5 h-3.5" />
                <span className="text-xs font-medium">{t('displays:unsupported_mode')}</span>
              </div>
            )}
          </div>
          <span className="text-xs font-mono text-muted-foreground">{display.deviceName}</span>
          {modeSummary && (
            <span className="text-xs text-muted-foreground mt-0.5">{modeSummary}</span>
          )}
        </div>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        className="h-7 w-7 text-muted-foreground hover:text-destructive-foreground hover:bg-destructive/10"
        onClick={onRemove}
        disabled={isDisabled}
        aria-label={t('common:remove')}
      >
        <Trash2 className="w-3.5 h-3.5" />
      </Button>
    </div>
  )
}

function ResolutionField({
  display,
  currentResKey,
  isDisabled,
  onChange
}: {
  display: DisplaySummary
  currentResKey: string
  isDisabled: boolean
  onChange: (v: string) => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-muted-foreground">{t('displays:resolution')}</label>
      <Select value={currentResKey} onValueChange={onChange} disabled={isDisabled}>
        <SelectTrigger className="h-8 text-xs bg-input border-border">
          <SelectValue placeholder={t('displays:resolution')} />
        </SelectTrigger>
        <SelectContent>
          {display.supportedResolutions.map((r) => {
            const key = `${r.width}x${r.height}`
            return (
              <SelectItem key={key} value={key} className="text-xs font-mono">
                {r.width}×{r.height}
              </SelectItem>
            )
          })}
        </SelectContent>
      </Select>
    </div>
  )
}

function RefreshRateField({
  currentHz,
  availableHz,
  isDisabled,
  onChange
}: {
  currentHz: number
  availableHz: number[]
  isDisabled: boolean
  onChange: (v: string) => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-muted-foreground">{t('displays:refresh_rate')}</label>
      <Select
        value={String(currentHz)}
        onValueChange={onChange}
        disabled={isDisabled || availableHz.length === 0}
      >
        <SelectTrigger className="h-8 text-xs bg-input border-border">
          <SelectValue placeholder={t('displays:refresh_rate')} />
        </SelectTrigger>
        <SelectContent>
          {availableHz.map((hz) => (
            <SelectItem key={hz} value={String(hz)} className="text-xs font-mono">
              {t('displays:hz', { value: hz })}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

function OrientationField({
  currentOrientation,
  isDisabled,
  onChange
}: {
  currentOrientation: Orientation
  isDisabled: boolean
  onChange: (v: string) => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-muted-foreground">{t('displays:orientation')}</label>
      <Select value={currentOrientation} onValueChange={onChange} disabled={isDisabled}>
        <SelectTrigger className="h-8 text-xs bg-input border-border">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {(Object.entries(ORIENTATION_KEYS) as [Orientation, string][]).map(([value, key]) => (
            <SelectItem key={value} value={value} className="text-xs">
              {t(key)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

export function DisplayCard({ display, driverOk }: DisplayCardProps): React.JSX.Element {
  const { currentResKey, availableHz, currentHz } = useDisplayCardModel(display)
  const setMode = useBridgeCommand(
    (input: Parameters<typeof window.easyVirtualDisplay.setDisplayMode>[0]) =>
      window.easyVirtualDisplay.setDisplayMode(input)
  )
  const removeDisplay = useBridgeCommand((index: number) =>
    window.easyVirtualDisplay.removeDisplay(index)
  )
  const busy = setMode.isPending || removeDisplay.isPending
  const isDisabled = !driverOk || busy

  const handleResolutionChange = (val: string): void => {
    const [w, h] = val.split('x').map(Number)
    const res = display.supportedResolutions.find((r) => r.width === w && r.height === h)
    if (!res) return
    const nextHz = res.refreshRates.includes(currentHz) ? currentHz : res.refreshRates[0]
    setMode.mutate({ index: display.index, width: w, height: h, hz: nextHz })
  }

  return (
    <div
      className={cn(
        'bg-card border border-border rounded-lg overflow-hidden transition-opacity',
        isDisabled && 'opacity-60 pointer-events-none'
      )}
    >
      <DisplayCardHeader
        display={display}
        isDisabled={isDisabled}
        onRemove={() => removeDisplay.mutate(display.index)}
      />
      <div className="h-px bg-border mx-4" />
      <div className="grid grid-cols-3 gap-3 px-4 py-3">
        <ResolutionField
          display={display}
          currentResKey={currentResKey}
          isDisabled={isDisabled}
          onChange={handleResolutionChange}
        />
        <RefreshRateField
          currentHz={currentHz}
          availableHz={availableHz}
          isDisabled={isDisabled}
          onChange={(val) => setMode.mutate({ index: display.index, hz: Number(val) })}
        />
        <OrientationField
          currentOrientation={display.currentOrientation}
          isDisabled={isDisabled}
          onChange={(val) =>
            setMode.mutate({ index: display.index, orientation: val as Orientation })
          }
        />
      </div>
    </div>
  )
}
