<p align="center">
  <img src="public/logo.png" alt="DeepSeek Harness Desktop" width="140">
</p>

<h1 align="center">DeepSeek Harness Desktop</h1>

<p align="center">
  <a href="./LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078D6.svg">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2.x-24C8DB.svg">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.77+-DEA584.svg">
</p>

<p align="center">
  <strong>English</strong> · <a href="./README.zh-CN.md">简体中文</a>
</p>
A lightweight Windows desktop built with [Tauri 2](https://tauri.app) that launches the
locally installed [DeepSeek Harness](https://www.npmjs.com/package/@deepseek-ai/dsh) and shows
its original web UI. It does not patch, copy, or rebuild DSH.

## Project Motivation

This project is intended for people who are not familiar with command-line tools, Node.js, or
local web-service setup. It provides a ready-to-use Windows desktop entry point that lowers the
barrier to installing and using DeepSeek Harness.

This project is a non-invasive desktop host, not a modified distribution of DSH. It does not alter
DSH source code, installed files, or runtime code; inject patches; copy DSH; or repackage it. The app
only uses the public `dsh` and `npm` commands to check the environment, install or update the official
package, verify versions, start the local service, and manage the DSH child process it started. On
uninstall, the global npm package is removed only when the app can confirm that it originally
installed DSH automatically; independently installed copies are left untouched.

## Features

- Checks the runtime environment (WebView2, Node.js, `dsh`) before starting
- Installs `dsh` automatically via npm when missing (npmmirror registry)
- Runs `dsh web --host 127.0.0.1 --port 0` and waits for the reported local URL
- Prompts for an in-app upgrade when a new DSH version is available
- Terminates the whole DSH process tree when the window is closed

## Requirements

| Dependency | Notes |
| --- | --- |
| Windows 10/11 | Windows only |
| Microsoft Edge WebView2 Runtime | Bootstrapped by the installer when missing |
| Node.js | Must be available on `PATH` |
| DeepSeek Harness (`dsh`) | Installed globally, `dsh --version` works |

## Install

Download the latest installer from the [Releases](https://github.com/wuxiaoji/Dsh_Desktop/releases)
page and run it.

## Build from Source

```powershell
npm install
npm run tauri build
```

The installer is generated at `src-tauri/target/release/bundle/nsis/`.

Each `npm run tauri build` bumps the patch version automatically, synced across
`package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, and
`src-tauri/Cargo.toml`:

```powershell
npm run tauri build            # 0.1.1 → 0.1.2 → 0.1.3 …
npm run tauri build -- minor   # 0.1.x → 0.2.0
npm run tauri build -- major   # 0.x.y → 1.0.0
```

`npm run tauri dev` and other tauri commands never touch the version.

## How It Works

1. Checks WebView2, Node.js, and the `dsh` CLI in sequence.
2. Installs `dsh` via npm if it is missing.
3. Runs `dsh web --host 127.0.0.1 --port 0` and parses the local URL from its output.
4. Verifies the TCP port is reachable, then navigates the window to the DSH UI.
5. Terminates the DSH process tree when the app exits.

## License

[MIT](./LICENSE) © 2026 wuxiaoji
