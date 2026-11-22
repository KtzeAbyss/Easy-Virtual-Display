import { Controller } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { ChevronDown, Plus, Shield, X } from 'lucide-react'
import type { DisplayMode } from '../../../shared'
import type { AdminConfigFormData } from '../../lib/schemas'
import type { useAdminConfigForm } from './useAdminConfigForm'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from '../ui/dropdown-menu'
import { SettingRow } from '../layout/Page'

type AdminForm = ReturnType<typeof useAdminConfigForm>

const MAX_CUSTOM_MODES = 5

const PRESET_MODES: DisplayMode[] = [
  { width: 1920, height: 1080, hz: 120 },
  { width: 2560, height: 1440, hz: 60 },
  { width: 2560, height: 1600, hz: 120 },
  { width: 3440, height: 1440, hz: 60 },
  { width: 3840, height: 2160, hz: 60 }
]

function ModeRow({
  index,
  register,
  invalid,
  onRemove
}: {
  index: number
  register: AdminForm['form']['register']
  invalid: boolean
  onRemove: () => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <li className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <Input
          type="number"
          placeholder={t('driver:placeholder_width')}
          aria-label={t('driver:placeholder_width')}
          aria-invalid={invalid || undefined}
          className="w-24 tabular-nums"
          {...register(`customModes.${index}.width`, { valueAsNumber: true })}
        />
        <span className="text-muted-foreground">×</span>
        <Input
          type="number"
          placeholder={t('driver:placeholder_height')}
          aria-label={t('driver:placeholder_height')}
          aria-invalid={invalid || undefined}
          className="w-24 tabular-nums"
          {...register(`customModes.${index}.height`, { valueAsNumber: true })}
        />
        <span className="text-muted-foreground">@</span>
        <Input
          type="number"
          placeholder="60"
          aria-label={t('driver:placeholder_hz')}
          aria-invalid={invalid || undefined}
          className="w-20 tabular-nums"
          {...register(`customModes.${index}.hz`, { valueAsNumber: true })}
        />
        <span className="text-muted-foreground">Hz</span>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="ml-auto text-muted-foreground"
          onClick={onRemove}
          aria-label={t('displays:remove')}
        >
          <X />
        </Button>
      </div>
      {invalid && <span className="text-xs text-status-error">{t('driver:invalid_values')}</span>}
    </li>
  )
}

function AddModeMenu({ onAdd }: { onAdd: (mode: DisplayMode) => void }): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="outline">
          <Plus />
          {t('driver:add_mode')}
          <ChevronDown className="size-3.5 text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        {PRESET_MODES.map((mode) => (
          <DropdownMenuItem
            key={`${mode.width}x${mode.height}@${mode.hz}`}
            className="tabular-nums"
            onSelect={() => onAdd(mode)}
          >
            {mode.width} × {mode.height} @ {mode.hz} Hz
          </DropdownMenuItem>
        ))}
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => onAdd({ width: 1920, height: 1080, hz: 60 })}>
          {t('driver:add_custom_mode')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function ParentGpuSelect({
  control,
  disabled
}: {
  control: AdminForm['form']['control']
  disabled: boolean
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <Controller<AdminConfigFormData, 'parentGpu'>
      control={control}
      name="parentGpu"
      render={({ field }) => (
        <Select value={field.value} onValueChange={field.onChange} disabled={disabled}>
          <SelectTrigger className="w-36" aria-label={t('driver:labels.parent_gpu')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="auto">{t('driver:gpu_options.auto')}</SelectItem>
            <SelectItem value="nvidia">{t('driver:gpu_options.nvidia')}</SelectItem>
            <SelectItem value="amd">{t('driver:gpu_options.amd')}</SelectItem>
          </SelectContent>
        </Select>
      )}
    />
  )
}

/** Driver-level configuration; every change is applied in one elevated step. */
export function CustomModesForm({ adminForm }: { adminForm: AdminForm }): React.JSX.Element {
  const { t } = useTranslation()
  const { form, fieldArray, onSubmit } = adminForm
  const { errors, isSubmitting, isDirty } = form.formState
  const { fields } = fieldArray

  return (
    <form className="flex flex-col gap-1" onSubmit={form.handleSubmit(onSubmit)}>
      <SettingRow
        label={t('driver:custom_resolutions')}
        description={t('driver:custom_modes_hint')}
        footer={
          <div className="border-t px-4 py-3">
            {fields.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('driver:no_custom_modes')}</p>
            ) : (
              <ul className="flex flex-col gap-2">
                {fields.map((field, i) => (
                  <ModeRow
                    key={field.id}
                    index={i}
                    register={form.register}
                    invalid={Boolean(errors.customModes?.[i])}
                    onRemove={() => fieldArray.remove(i)}
                  />
                ))}
              </ul>
            )}
            {errors.customModes?.message && (
              <p className="mt-2 text-xs text-status-error">{errors.customModes.message}</p>
            )}
          </div>
        }
      >
        <span className="text-xs text-muted-foreground tabular-nums">
          {fields.length} / {MAX_CUSTOM_MODES}
        </span>
        {fields.length < MAX_CUSTOM_MODES && (
          <AddModeMenu onAdd={(mode) => fieldArray.append(mode)} />
        )}
      </SettingRow>
      <SettingRow
        label={t('driver:labels.parent_gpu')}
        description={t('driver:gpu_selection_desc')}
      >
        <ParentGpuSelect control={form.control} disabled={isSubmitting} />
      </SettingRow>
      <div className="flex items-center justify-end gap-2 pt-2">
        <span className="mr-auto text-xs text-muted-foreground">{t('driver:uac_note')}</span>
        {isDirty && (
          <Button
            type="button"
            variant="outline"
            disabled={isSubmitting}
            onClick={() => form.reset()}
          >
            {t('driver:discard')}
          </Button>
        )}
        <Button type="submit" disabled={!isDirty || isSubmitting} className="min-w-24">
          <Shield />
          {isSubmitting ? t('driver:applying') : t('driver:apply')}
        </Button>
      </div>
    </form>
  )
}
