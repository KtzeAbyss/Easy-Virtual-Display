import { describe, expect, test } from 'vitest'
import enShell from '../en/shell.json'
import zhShell from '../zh-CN/shell.json'
import enCommon from '../en/common'
import zhCommon from '../zh-CN/common'
import enTray from '../en/tray'
import zhTray from '../zh-CN/tray'

// shell.json holds the small subset of strings the Tauri Rust shell needs (tray menu +
// install-driver / quit dialogs). It must be the single source of truth so the Rust
// `include_str!`-ed JSON cannot drift from what the renderer sees.

function sortedKeys(o: Record<string, unknown>): string[] {
  return Object.keys(o).sort()
}

const SHELL_NAMESPACES = ['tray', 'common'] as const

describe('shell.json locale parity', () => {
  test('en and zh-CN expose the same shell namespaces', () => {
    expect(sortedKeys(enShell)).toEqual(sortedKeys(zhShell))
    expect(sortedKeys(enShell).sort()).toEqual([...SHELL_NAMESPACES].sort())
  })

  for (const ns of SHELL_NAMESPACES) {
    test(`en and zh-CN have identical keys under shell.${ns}`, () => {
      const enKeys = sortedKeys(enShell[ns] as Record<string, string>)
      const zhKeys = sortedKeys(zhShell[ns] as Record<string, string>)
      expect(zhKeys).toEqual(enKeys)
      expect(enKeys.length).toBeGreaterThan(0)
    })
  }

  test('shell.json values are non-empty strings', () => {
    for (const ns of SHELL_NAMESPACES) {
      for (const [k, v] of Object.entries(enShell[ns] as Record<string, string>)) {
        expect(typeof v, `en/shell.${ns}.${k}`).toBe('string')
        expect((v as string).length, `en/shell.${ns}.${k}`).toBeGreaterThan(0)
      }
      for (const [k, v] of Object.entries(zhShell[ns] as Record<string, string>)) {
        expect(typeof v, `zh/shell.${ns}.${k}`).toBe('string')
        expect((v as string).length, `zh/shell.${ns}.${k}`).toBeGreaterThan(0)
      }
    }
  })

  test('shell.json values flow into the i18next resources via the TS adapters', () => {
    // tray.ts re-exports shell.json[tray]; identity check ensures no transformation drift.
    expect(enTray).toStrictEqual(enShell.tray)
    expect(zhTray).toStrictEqual(zhShell.tray)

    // common.ts spreads shell.common first, then layers renderer-only keys on top. Every
    // shell.common key must therefore be present in the merged result with the JSON value.
    for (const [k, v] of Object.entries(enShell.common)) {
      expect(enCommon, `en/common.${k}`).toHaveProperty(k, v)
    }
    for (const [k, v] of Object.entries(zhShell.common)) {
      expect(zhCommon, `zh/common.${k}`).toHaveProperty(k, v)
    }
  })

  test('shell-required tray and common keys cover the strings the Rust shell uses', () => {
    // These are the keys referenced by src-tauri/src/tray.rs, install_prompt.rs and quit.rs.
    // The test fails fast if a Rust call site introduces a new key without adding it to
    // shell.json (catching drift at the source).
    const requiredTray = [
      'show',
      'add_display',
      'remove_last_display',
      'keep_screen_on',
      'launch_on_login',
      'quit',
      'tooltip_active',
      'tooltip_inactive',
      'quit_title',
      'quit_message',
      'quit_detail',
      'quit_detail_plural',
      'quit_button',
      'cancel'
    ]
    const requiredCommon = [
      'cancel',
      'install_driver_action',
      'install_driver_title',
      'install_driver_message',
      'install_driver_detail'
    ]
    for (const k of requiredTray) {
      expect(enShell.tray, `en/shell.tray.${k}`).toHaveProperty(k)
      expect(zhShell.tray, `zh/shell.tray.${k}`).toHaveProperty(k)
    }
    for (const k of requiredCommon) {
      expect(enShell.common, `en/shell.common.${k}`).toHaveProperty(k)
      expect(zhShell.common, `zh/shell.common.${k}`).toHaveProperty(k)
    }
  })
})
