import { z } from 'zod'

export const displayModeSchema = z.object({
  width: z.number().int().positive(),
  height: z.number().int().positive(),
  hz: z.number().int().positive()
})

export const adminConfigSchema = z.object({
  customModes: z.array(displayModeSchema).max(5),
  parentGpu: z.enum(['auto', 'nvidia', 'amd'])
})

export type AdminConfigFormData = z.infer<typeof adminConfigSchema>
