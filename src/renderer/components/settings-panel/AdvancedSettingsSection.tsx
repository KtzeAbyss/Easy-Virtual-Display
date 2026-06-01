import { Controller } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import type { AdminConfigFormData } from '../../lib/schemas'
import type { useAdminConfigForm } from './useAdminConfigForm'
import { Input } from '../ui/input'
import {
  SectionTitle,
  SettingRow,
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Shield,
  Plus,
  Trash2,
  Save
} from './SettingsPrimitives'

interface AdvancedSettingsSectionProps {
  adminForm: ReturnType<typeof useAdminConfigForm>
}

function ModeRow({
  index,
  field,
  register,
  errors,
  onRemove
}: {
  index: number
  field: { id: string }
  register: ReturnType<typeof useAdminConfigForm>['form']['register']
  errors: ReturnType<typeof useAdminConfigForm>['form']['formState']['errors']
  onRemove: () => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div
      key={field.id}
      className="flex items-center justify-between gap-2 px-2.5 py-1.5 bg-muted rounded-md"
    >
      <div className="flex items-center gap-2">
        <Input
          type="number"
          placeholder={t('displays:placeholder_width')}
          className="h-7 w-20 text-xs font-mono bg-input border-border"
          {...register(`customModes.${index}.width`, { valueAsNumber: true })}
        />
        <span className="text-muted-foreground text-xs">×</span>
        <Input
          type="number"
          placeholder={t('displays:placeholder_height')}
          className="h-7 w-20 text-xs font-mono bg-input border-border"
          {...register(`customModes.${index}.height`, { valueAsNumber: true })}
        />
        <span className="text-muted-foreground text-xs">@</span>
        <Input
          type="number"
          placeholder={t('displays:placeholder_hz')}
          className="h-7 w-16 text-xs font-mono bg-input border-border"
          {...register(`customModes.${index}.hz`, { valueAsNumber: true })}
        />
      </div>
      <button
        type="button"
        onClick={onRemove}
        className="text-muted-foreground hover:text-destructive-foreground transition-colors"
        aria-label={t('common:remove')}
      >
        <Trash2 className="w-3.5 h-3.5" />
      </button>
      {errors.customModes?.[index] && (
        <span className="block text-xs text-destructive">{t('displays:invalid_values')}</span>
      )}
    </div>
  )
}

function CustomModesEditor({
  fields,
  register,
  errors,
  onAppend,
  onRemove
}: {
  fields: { id: string }[]
  register: ReturnType<typeof useAdminConfigForm>['form']['register']
  errors: ReturnType<typeof useAdminConfigForm>['form']['formState']['errors']
  onAppend: () => void
  onRemove: (i: number) => void
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="px-4 py-3">
      <div className="flex items-center justify-between mb-2">
        <div>
          <p className="text-sm text-foreground">{t('displays:custom_resolutions')}</p>
          <p className="text-xs text-muted-foreground">{t('displays:custom_modes_hint')}</p>
        </div>
        <span className="text-xs font-mono text-muted-foreground">
          {t('settings:custom_modes_count', { count: fields.length })}
        </span>
      </div>
      <div className="flex flex-col gap-1.5 mb-2">
        {fields.map((field, i) => (
          <ModeRow
            key={field.id}
            index={i}
            field={field}
            register={register}
            errors={errors}
            onRemove={() => onRemove(i)}
          />
        ))}
      </div>
      {fields.length < 5 && (
        <Button type="button" variant="outline" size="sm" className="gap-1.5" onClick={onAppend}>
          <Plus className="w-3.5 h-3.5" />
          {t('displays:add_mode')}
        </Button>
      )}
      {errors.customModes?.message && (
        <p className="block text-xs text-destructive mt-1">{errors.customModes.message}</p>
      )}
    </div>
  )
}

function ParentGpuField({
  control,
  isSubmitting
}: {
  control: ReturnType<typeof useAdminConfigForm>['form']['control']
  isSubmitting: boolean
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="px-4">
      <SettingRow
        label={t('settings:gpu_selection')}
        description={t('settings:gpu_selection_desc')}
      >
        <Controller<AdminConfigFormData, 'parentGpu'>
          control={control}
          name="parentGpu"
          render={({ field }) => (
            <Select value={field.value} onValueChange={field.onChange} disabled={isSubmitting}>
              <SelectTrigger className="w-32 h-8 text-xs bg-input border-border">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="auto" className="text-xs">
                  {t('driver:gpu_options.auto')}
                </SelectItem>
                <SelectItem value="nvidia" className="text-xs">
                  {t('driver:gpu_options.nvidia')}
                </SelectItem>
                <SelectItem value="amd" className="text-xs">
                  {t('driver:gpu_options.amd')}
                </SelectItem>
              </SelectContent>
            </Select>
          )}
        />
      </SettingRow>
    </div>
  )
}

function AdminConfigNotice({ isSubmitting }: { isSubmitting: boolean }): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="px-4 py-3">
      <div className="flex items-start gap-3 p-3 bg-status-warn/5 border border-status-warn/20 rounded-md mb-3">
        <Shield className="w-4 h-4 text-status-warn mt-0.5 shrink-0" />
        <p className="text-xs text-muted-foreground leading-relaxed">{t('settings:uac_warning')}</p>
      </div>
      <Button type="submit" className="w-full gap-2" disabled={isSubmitting}>
        <Save className="w-3.5 h-3.5" />
        {isSubmitting ? t('common:saving') : t('settings:save_config')}
      </Button>
    </div>
  )
}

export function AdvancedSettingsSection({
  adminForm
}: AdvancedSettingsSectionProps): React.JSX.Element {
  const { t } = useTranslation()
  const { form, fieldArray, onSubmit } = adminForm
  const { errors, isSubmitting } = form.formState

  return (
    <section>
      <SectionTitle>{t('settings:advanced_title')}</SectionTitle>
      <form onSubmit={form.handleSubmit(onSubmit)}>
        <div className="bg-card border border-border rounded-lg divide-y divide-border">
          <CustomModesEditor
            fields={fieldArray.fields}
            register={form.register}
            errors={errors}
            onAppend={() => fieldArray.append({ width: 1920, height: 1080, hz: 60 })}
            onRemove={(i) => fieldArray.remove(i)}
          />
          <ParentGpuField control={form.control} isSubmitting={isSubmitting} />
          <AdminConfigNotice isSubmitting={isSubmitting} />
        </div>
      </form>
    </section>
  )
}
