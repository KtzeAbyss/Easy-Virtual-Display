import { describe, it, expect } from 'vitest'
import { resources } from '../../shared/locales'

function collectKeys(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return []
  const record = obj as Record<string, unknown>
  const keys: string[] = []
  for (const key of Object.keys(record).sort()) {
    const full = prefix ? `${prefix}.${key}` : key
    if (record[key] !== null && typeof record[key] === 'object') {
      keys.push(...collectKeys(record[key], full))
    } else {
      keys.push(full)
    }
  }
  return keys
}

const locales = Object.keys(resources) as (keyof typeof resources)[]
const namespaces = Object.keys(resources.en) as (keyof typeof resources.en)[]

describe('locale parity', () => {
  for (const ns of namespaces) {
    it(`${ns}: all locales have identical key structure`, () => {
      const enKeys = collectKeys(resources.en[ns])
      expect(enKeys.length, `en/${ns} should have keys`).toBeGreaterThan(0)

      for (const locale of locales) {
        if (locale === 'en') continue
        const localeKeys = collectKeys(resources[locale][ns])
        expect(localeKeys).toEqual(enKeys)
      }
    })
  }
})
