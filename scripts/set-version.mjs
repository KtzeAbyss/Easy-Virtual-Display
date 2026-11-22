#!/usr/bin/env node
/**
 * Keep package.json, package-lock.json, tauri.conf.json, Cargo.toml and Cargo.lock
 * on the same app version.
 *
 *   node scripts/set-version.mjs 0.2.2
 *   node scripts/set-version.mjs --bump patch|minor|major|prerelease
 *   node scripts/set-version.mjs --check [expected]
 *
 * Prints the resulting / current version to stdout.
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')

function readJson(rel) {
  return JSON.parse(readFileSync(resolve(root, rel), 'utf8'))
}

function writeJson(rel, value) {
  writeFileSync(resolve(root, rel), `${JSON.stringify(value, null, 2)}\n`)
}

/** Strictish SemVer: major.minor.patch with an optional -prerelease. */
function parse(version) {
  const m = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][\w-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][\w-]*))*))?$/.exec(
    version
  )
  if (!m) {
    throw new Error(`Invalid SemVer version: ${version}`)
  }
  return {
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    prerelease: m[4] ?? null
  }
}

function format({ major, minor, patch, prerelease }) {
  return prerelease == null
    ? `${major}.${minor}.${patch}`
    : `${major}.${minor}.${patch}-${prerelease}`
}

function bump(version, kind) {
  const v = parse(version)
  switch (kind) {
    case 'major':
      return format({ major: v.major + 1, minor: 0, patch: 0, prerelease: null })
    case 'minor':
      return format({ major: v.major, minor: v.minor + 1, patch: 0, prerelease: null })
    case 'patch':
      if (v.prerelease != null) {
        // Drop the prerelease label and keep the same numbers (beta → stable).
        return format({ ...v, prerelease: null })
      }
      return format({ ...v, patch: v.patch + 1, prerelease: null })
    case 'prerelease': {
      if (v.prerelease == null) {
        return format({ ...v, patch: v.patch + 1, prerelease: 'beta.1' })
      }
      const parts = v.prerelease.split('.')
      const last = parts.at(-1)
      if (/^\d+$/.test(last)) {
        parts[parts.length - 1] = String(Number(last) + 1)
      } else {
        parts.push('1')
      }
      return format({ ...v, prerelease: parts.join('.') })
    }
    default:
      throw new Error(`Unknown bump kind: ${kind}`)
  }
}

function apply(version) {
  parse(version) // validate

  const pkg = readJson('package.json')
  pkg.version = version
  writeJson('package.json', pkg)

  const lock = readJson('package-lock.json')
  lock.version = version
  if (lock.packages?.['']) {
    lock.packages[''].version = version
  }
  writeJson('package-lock.json', lock)

  const conf = readJson('src-tauri/tauri.conf.json')
  conf.version = version
  writeJson('src-tauri/tauri.conf.json', conf)

  const cargoPath = resolve(root, 'src-tauri/Cargo.toml')
  const cargo = readFileSync(cargoPath, 'utf8').replace(
    /^version = "[^"]+"/m,
    `version = "${version}"`
  )
  writeFileSync(cargoPath, cargo)

  const lockPath = resolve(root, 'src-tauri/Cargo.lock')
  const cargoLock = readFileSync(lockPath, 'utf8').replace(
    /(name = "easy-virtual-display"\n)version = "[^"]+"/,
    `$1version = "${version}"`
  )
  writeFileSync(lockPath, cargoLock)
}

function readCargoVersion() {
  const m = /^version = "([^"]+)"/m.exec(
    readFileSync(resolve(root, 'src-tauri/Cargo.toml'), 'utf8')
  )
  if (!m) throw new Error('Could not read version from src-tauri/Cargo.toml')
  return m[1]
}

function currentVersions() {
  return {
    package: readJson('package.json').version,
    tauri: readJson('src-tauri/tauri.conf.json').version,
    cargo: readCargoVersion()
  }
}

function check(expected) {
  const versions = currentVersions()
  const values = Object.values(versions)
  const unique = [...new Set(values)]
  if (unique.length !== 1) {
    throw new Error(
      `Version mismatch across files: package.json=${versions.package} tauri.conf.json=${versions.tauri} Cargo.toml=${versions.cargo}`
    )
  }
  if (expected != null && unique[0] !== expected) {
    throw new Error(`Expected version ${expected}, found ${unique[0]}`)
  }
  return unique[0]
}

function resolveTarget(argv) {
  if (argv[0] === '--check') {
    const expected = argv[1] ? argv[1].replace(/^v/, '') : null
    return { mode: 'check', version: check(expected) }
  }

  const bumpIdx = argv.indexOf('--bump')
  if (bumpIdx !== -1) {
    const kind = argv[bumpIdx + 1]
    if (!kind || kind.startsWith('-')) {
      throw new Error('Usage: --bump patch|minor|major|prerelease')
    }
    return { mode: 'apply', version: bump(readJson('package.json').version, kind) }
  }

  const explicit = argv.find((a) => !a.startsWith('-'))
  if (!explicit) {
    throw new Error(
      'Usage: node scripts/set-version.mjs <version> | --bump patch|minor|major|prerelease | --check [expected]'
    )
  }
  return { mode: 'apply', version: explicit.replace(/^v/, '') }
}

const { mode, version } = resolveTarget(process.argv.slice(2))
if (mode === 'apply') {
  apply(version)
}
process.stdout.write(`${version}\n`)
