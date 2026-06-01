# Easy Virtual Display

**English** · [简体中文](./README.zh-CN.md)

> Add, remove, and manage virtual monitors on Windows from a clean native desktop app — no command line, no Parsec account.

Easy Virtual Display is a Windows desktop application for installing and driving a **virtual display driver** through a friendly UI. A virtual display behaves like a real monitor that isn't physically attached to your machine — perfect for headless servers and GPU rigs, remote desktop and streaming, giving a laptop extra screen real estate, or testing multi-monitor layouts.

It builds on the early concept of [nomi-san/parsec-vdd](https://github.com/nomi-san/parsec-vdd).

> 📦 Prebuilt installers are available on the [Releases](https://github.com/KtzeAbyss/Easy-Virtual-Display/releases) page.

## Highlights

- **One-click driver install** — the driver is bundled; install it with a single UAC-elevated click, no separate download.
- **Add & remove displays anywhere** — from the main window or the system tray, up to 8 virtual displays.
- **Per-display control** — set resolution, refresh rate, and orientation (landscape / portrait / flipped) for each virtual monitor.
- **Custom resolution modes** — define up to 5 custom modes the driver should advertise.
- **GPU targeting** — pin virtual displays to a specific parent GPU (Auto / NVIDIA / AMD).
- **At-a-glance status** — driver state, driver version, max displays, and active display count.
- **Smart behaviors** — launch on login, close-to-tray, start minimized, *keep screen on* while a display is active, and a *fallback display* that auto-appears when your primary monitor disconnects.
- **Polished UX** — light / dark / system themes and an English / 简体中文 interface.

## How it works

Easy Virtual Display is split into three cooperating layers:

| Layer | Tech | Responsibility |
| --- | --- | --- |
| **Shell** | Tauri 2 (Rust) + system WebView2 | Window, system tray, OS integrations (autostart, keep-awake, fallback display), driver lifecycle, and privilege elevation. |
| **Renderer** | React 19 + TypeScript (Vite) | The desktop UI — TanStack Query, react-hook-form + Zod, Tailwind CSS, Radix primitives, and i18next. |
| **Native host** | .NET (C#) | Talks to the virtual display driver over `DeviceIoControl` / SetupAPI / CfgMgr32 and exposes it to the shell via a stdio JSON-RPC protocol. |

Shared TypeScript contracts in `src/shared` keep the renderer and shell in lock-step. Privileged actions (install/uninstall the driver, write custom modes) are routed through a dedicated elevated host invocation so the main app never runs with more rights than it needs.

## Project layout

```
src/                 React renderer + shared TypeScript contracts
  renderer/          UI components, hooks, i18n
  shared/            contracts, IPC, locales (en, zh-CN)
src-tauri/           Tauri (Rust) shell — commands, tray, OS boundaries
native/              .NET host + virtual display control core
  EasyVirtualDisplay.Host/   stdio JSON-RPC host & admin CLI
  EasyVirtualDisplay.Vdd/    driver interop (DeviceIoControl / SetupAPI)
vendor/parsec-vdd/   bundled virtual display driver installer
```

## Getting started

### Prerequisites

- **Windows** (the driver and packaging are Windows-only)
- **Node.js** and **npm**
- **Rust** toolchain (stable) — for the Tauri shell
- **.NET SDK** — for the native host

### Install dependencies

```bash
npm install
```

### Run in development

```bash
npm run tauri:dev
```

### Checks

```bash
npm run typecheck
npm run test
```

### Build the Windows installer

```bash
npm run tauri:build
```

This publishes the .NET host and produces an NSIS installer that bundles the renderer, the Rust shell, and the driver.

## Repository

- GitHub: https://github.com/KtzeAbyss/Easy-Virtual-Display
