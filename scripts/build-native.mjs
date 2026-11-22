// Publish the .NET host. Currently only win-x64 is supported (Parsec VDD is x64-only).
// DOTNET_RID may be set explicitly; anything other than win-x64 fails fast.
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const rid = process.env.DOTNET_RID || 'win-x64'
if (rid !== 'win-x64') {
  console.error(
    `Unsupported DOTNET_RID="${rid}". Only win-x64 is supported until a non-x64 driver backend lands.`
  )
  process.exit(1)
}

const root = resolve(fileURLToPath(import.meta.url), '..', '..')
const project = resolve(root, 'native/EasyVirtualDisplay.Host/EasyVirtualDisplay.Host.csproj')
const output = resolve(root, 'native/EasyVirtualDisplay.Host/bin/publish')

const args = [
  'publish',
  project,
  '-c',
  'Release',
  '-r',
  rid,
  '--self-contained',
  'false',
  '-o',
  output
]

console.log(`Publishing native host (${rid}) → ${output}`)
const result = spawnSync('dotnet', args, { stdio: 'inherit', shell: false })
process.exit(result.status ?? 1)
