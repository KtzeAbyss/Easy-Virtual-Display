import { displayModeSchema, adminConfigSchema } from '../lib/schemas'

describe('displayModeSchema', () => {
  it('accepts valid positive integer mode', () => {
    expect(displayModeSchema.safeParse({ width: 1920, height: 1080, hz: 60 }).success).toBe(true)
  })

  it('rejects zero width', () => {
    expect(displayModeSchema.safeParse({ width: 0, height: 1080, hz: 60 }).success).toBe(false)
  })

  it('rejects negative height', () => {
    expect(displayModeSchema.safeParse({ width: 1920, height: -1, hz: 60 }).success).toBe(false)
  })

  it('rejects non-integer hz', () => {
    expect(displayModeSchema.safeParse({ width: 1920, height: 1080, hz: 59.94 }).success).toBe(
      false
    )
  })
})

describe('adminConfigSchema', () => {
  it('accepts valid config with no custom modes', () => {
    const result = adminConfigSchema.safeParse({ customModes: [], parentGpu: 'auto' })
    expect(result.success).toBe(true)
  })

  it('accepts up to 5 custom modes', () => {
    const modes = Array.from({ length: 5 }, () => ({ width: 1920, height: 1080, hz: 60 }))
    expect(adminConfigSchema.safeParse({ customModes: modes, parentGpu: 'nvidia' }).success).toBe(
      true
    )
  })

  it('rejects more than 5 custom modes', () => {
    const modes = Array.from({ length: 6 }, () => ({ width: 1920, height: 1080, hz: 60 }))
    expect(adminConfigSchema.safeParse({ customModes: modes, parentGpu: 'auto' }).success).toBe(
      false
    )
  })

  it('rejects invalid parentGpu', () => {
    expect(adminConfigSchema.safeParse({ customModes: [], parentGpu: 'intel' }).success).toBe(false)
  })
})
