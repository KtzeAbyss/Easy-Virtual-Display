// Fetch the Parsec Virtual Display Driver installer that the app bundles. The file is not
// committed (see .gitignore); the pinned hash makes every build ship the same bytes and
// fails the build if the download ever changes.
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const SOURCE = 'https://builds.parsec.app/vdd/parsec-vdd-0.45.0.0.exe'
const SHA256 = 'e23332448fdaf5aa017cb308db5ef6855fac526a7ded05d80c039404126d5362'
const TARGET = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../vendor/parsec-vdd/parsec-vdd-0.45.0.0.exe'
)

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

if (existsSync(TARGET) && sha256(readFileSync(TARGET)) === SHA256) {
  console.log(`Driver installer already present: ${TARGET}`)
  process.exit(0)
}

const response = await fetch(SOURCE)
if (!response.ok) {
  throw new Error(`Downloading ${SOURCE} failed: HTTP ${response.status}`)
}
const bytes = Buffer.from(await response.arrayBuffer())
const actual = sha256(bytes)
if (actual !== SHA256) {
  throw new Error(`${SOURCE} has SHA-256 ${actual}, expected ${SHA256}`)
}

mkdirSync(dirname(TARGET), { recursive: true })
writeFileSync(TARGET, bytes)
console.log(`Downloaded driver installer: ${TARGET}`)
