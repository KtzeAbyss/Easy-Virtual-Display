import { useEffect } from 'react'
import {
  useFieldArray,
  useForm,
  type UseFormReturn,
  type UseFieldArrayReturn
} from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  EASY_VIRTUAL_DISPLAY_ERROR_CODES,
  type EasyVirtualDisplayErrorCode,
  type HostSnapshot,
  type DisplayMode
} from '../../../shared'
import { adminConfigSchema, type AdminConfigFormData } from '../../lib/schemas'

function mapCustomModes(host: HostSnapshot): DisplayMode[] {
  return host.customModes.map((m) => ({ width: m.width, height: m.height, hz: m.hz }))
}

interface AdminConfigFormResult {
  form: UseFormReturn<AdminConfigFormData>
  fieldArray: UseFieldArrayReturn<AdminConfigFormData, 'customModes'>
  onSubmit: (data: AdminConfigFormData) => Promise<void>
}

export function useAdminConfigForm(host: HostSnapshot): AdminConfigFormResult {
  const { t } = useTranslation()

  const form = useForm<AdminConfigFormData>({
    resolver: zodResolver(adminConfigSchema),
    defaultValues: {
      customModes: mapCustomModes(host),
      parentGpu: host.parentGpu
    }
  })

  const fieldArray = useFieldArray({ control: form.control, name: 'customModes' })

  useEffect(() => {
    if (!form.formState.isDirty) {
      form.reset({ customModes: mapCustomModes(host), parentGpu: host.parentGpu })
    }
  }, [host.customModes, host.parentGpu, form.reset, form.formState.isDirty, form])

  const onSubmit = async (data: AdminConfigFormData): Promise<void> => {
    try {
      await window.easyVirtualDisplay.applyAdminConfig(data)
      form.reset(data)
      toast.success(t('displays:config_saved'))
    } catch (e: unknown) {
      const code = (e as Record<string, unknown>)?.code
      if (code === 'admin_cancelled') {
        toast.info(t('common:errors.admin_cancelled'))
      } else if (code === 'config_apply_timeout') {
        form.reset(data)
        toast.warning(t('common:errors.config_apply_timeout'))
      } else if (EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)) {
        toast.error(t(`common:errors.${code}`))
      } else {
        toast.error(t('common:operation_failed'))
      }
    }
  }

  return { form, fieldArray, onSubmit }
}
