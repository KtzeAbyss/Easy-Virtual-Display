# Easy Virtual Display

[English](./README.md) · **简体中文**

> 在 Windows 上通过简洁的原生桌面应用添加、移除和管理虚拟显示器——无需命令行，也不需要 Parsec 账号。

Easy Virtual Display 是一款 Windows 桌面应用，让你通过友好的界面安装并驱动一块**虚拟显示驱动**。虚拟显示器在系统看来就像一块真实存在、却没有物理连接的显示器——非常适合无头服务器与 GPU 主机、远程桌面与串流、为笔记本扩展额外的屏幕空间，或测试多显示器布局。

本项目基于 [nomi-san/parsec-vdd](https://github.com/nomi-san/parsec-vdd) 的早期概念发展而来。

> 📦 预构建的安装包可在 [Releases](https://github.com/KtzeAbyss/Easy-Virtual-Display/releases) 页面下载。

## 功能亮点

- **一键安装驱动** —— 驱动已随应用内置，UAC 提权后一键安装，无需另行下载。
- **随处增删显示器** —— 在主窗口或系统托盘中操作，最多可创建 8 块虚拟显示器。
- **逐屏精细控制** —— 为每块虚拟显示器分别设置分辨率、刷新率和方向（横向 / 纵向 / 翻转）。
- **自定义分辨率模式** —— 最多可定义 5 组驱动对外提供的自定义模式。
- **指定 GPU** —— 将虚拟显示器绑定到指定的父级 GPU（自动 / NVIDIA / AMD）。
- **状态一目了然** —— 驱动状态、驱动版本、最大显示器数量以及当前活动显示器数。
- **贴心的行为设置** —— 开机自启、关闭到托盘、启动时隐藏到托盘、显示器活动时*保持屏幕常亮*，以及没有任何显示器工作时自动顶上的*兜底显示器*。
- **命令行控制** —— 用 `evdctl` 在脚本里增删显示器，比如作为 Sunshine 的准备命令。
- **精致的体验** —— 浅色 / 深色 / 跟随系统主题，界面支持 English / 简体中文。

## 命令行

安装程序会在 `EasyVirtualDisplay.exe` 旁边放一个 `evdctl.cmd`（默认位于 `%LOCALAPPDATA%\EasyVirtualDisplay`）。它请求正在运行的主程序增删显示器，并以退出码返回结果；主程序没有运行时，会先在托盘中启动它。

```bat
evdctl ensure --key <key> [--wait [秒数]]   :: 为 <key> 添加一块显示器，已有则不再添加
evdctl release --key <key>                  :: 移除属于 <key> 的显示器，没有则什么也不做
evdctl status                               :: 以 JSON 输出驱动状态和显示器列表
```

key 保证 `release` 只移除对应 `ensure` 添加的那块显示器，不会动你自己添加的。`--wait` 会等到显示器真正启用（默认 15 秒）。退出码：`0` 成功，`1` 用法错误，`2` 主程序不可用，`3` 操作失败，`4` 等待超时。

请以与主程序相同的 Windows 用户运行 `evdctl`；以管理员身份运行也可以。

### Sunshine

如果只想在串流时才有虚拟显示器，可在 Sunshine 网页界面里添加准备命令（全局设置在 *General* 中，也可以按应用单独设置）：

| | 命令 |
| --- | --- |
| Do | `"C:\Users\<你的用户名>\AppData\Local\EasyVirtualDisplay\evdctl.cmd" ensure --key sunshine --wait` |
| Undo | `"C:\Users\<你的用户名>\AppData\Local\EasyVirtualDisplay\evdctl.cmd" release --key sunshine` |

`evdctl` 只负责添加和移除显示器。要串流这块显示器或把它设为主显示器，请使用 Sunshine 自身的显示器设置或其他工具。

## 工作原理

Easy Virtual Display 由三个协同工作的层次组成：

| 层次 | 技术 | 职责 |
| --- | --- | --- |
| **外壳** | Tauri 2（Rust）+ 系统 WebView2 | 窗口、系统托盘、系统集成（自启、防休眠、兜底显示器）、驱动生命周期与权限提升。 |
| **渲染层** | React 19 + TypeScript（Vite） | 桌面界面——TanStack Query、react-hook-form + Zod、Tailwind CSS、Radix 组件与 i18next。 |
| **原生宿主** | .NET（C#） | 通过 `DeviceIoControl` / SetupAPI / CfgMgr32 与虚拟显示驱动通信，并经由 stdio JSON-RPC 协议暴露给外壳、经由命名管道暴露给 `evdctl`。 |

`src/shared` 中共享的 TypeScript 契约让渲染层与外壳保持严格同步。需要提权的操作（安装/卸载驱动、写入自定义模式）会被路由到专门的提权宿主调用——主程序本身始终以最小权限运行，绝不多取一分。

## 项目结构

```
src/                 React 渲染层 + 共享 TypeScript 契约
  renderer/          UI 组件、hooks、国际化
  shared/            契约、IPC、语言包（en、zh-CN）
src-tauri/           Tauri（Rust）外壳——命令、托盘、系统边界
native/              .NET 宿主 + 虚拟显示控制核心
  EasyVirtualDisplay.Host/   stdio JSON-RPC 宿主、管理员 CLI 与 evdctl 管道
  EasyVirtualDisplay.Vdd/    驱动互操作（DeviceIoControl / SetupAPI）
vendor/parsec-vdd/   内置的虚拟显示驱动安装包
```

## 快速开始

### 环境要求

- **Windows**（驱动与打包仅支持 Windows）
- **Node.js** 与 **npm**
- **Rust** 工具链（stable）—— 用于 Tauri 外壳
- **.NET SDK** —— 用于原生宿主

### 安装依赖

```bash
npm install
```

### 开发模式运行

```bash
npm run fetch:vdd   # 首次运行：把驱动安装包下载到 vendor/parsec-vdd/
npm run tauri:dev
```

### 检查

```bash
npm run typecheck
npm run test
```

### 构建 Windows 安装包

```bash
npm run tauri:build
```

该命令会获取驱动安装包（按固定的 SHA-256 校验）、发布 .NET 宿主，并生成一个 NSIS 安装包，其中打包了渲染层、Rust 外壳与驱动。

## 仓库

- GitHub：https://github.com/KtzeAbyss/Easy-Virtual-Display
