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
  <a href="./README.md">English</a> · <strong>简体中文</strong>
</p>

基于 [Tauri 2](https://tauri.app) 的轻量 Windows 桌面版，用于启动本机安装的
[DeepSeek Harness](https://www.npmjs.com/package/@deepseek-ai/dsh) 并展示其原始 Web 界面。
本项目不修改、不复制、不重新构建 DSH。

## 项目初衷

本项目面向不熟悉命令行、Node.js 或本地 Web 服务配置的普通用户，希望通过一个开箱即用的
Windows 桌面入口，降低安装和使用 DeepSeek Harness 的门槛。

本项目是一个非侵入式桌面宿主，而不是 DSH 的修改版：不会修改 DSH 源码、安装文件或运行时代码，
不会向 DSH 注入补丁，也不会复制或重新打包 DSH。应用只通过公开的 `dsh` 和 `npm` 命令完成环境
检查、官方包安装或更新、版本校验与本地服务启动，并管理由自身启动的 DSH 子进程。卸载本应用时，
只有确认 DSH 最初由本应用自动安装，才会清理对应的全局 npm 包；用户自行安装的 DSH 不受影响。

## 特性

- 启动前检查运行环境（WebView2、Node.js、`dsh`）
- 未安装 `dsh` 时自动通过 npm 安装（使用 npmmirror 镜像源）
- 执行 `dsh web --host 127.0.0.1 --port 0`，等待其输出的本地地址
- 启动时通过阿里云 npm 镜像严格校验 DSH 最新版本
- 检测到新版本时必须选择立即更新或暂不更新，选择前不会继续启动
- 关闭窗口时终止整个 DSH 进程树

## 环境要求

| 依赖 | 说明 |
| --- | --- |
| Windows 10/11 | 仅支持 Windows |
| Microsoft Edge WebView2 Runtime | 缺失时由安装包自动引导安装 |
| Node.js | 需加入 `PATH` |
| DeepSeek Harness（`dsh`） | 全局安装，`dsh --version` 可用 |

## 安装

从 [Releases](https://github.com/wuxiaoji/Dsh_Desktop/releases) 页面下载最新安装包并运行。

## 从源码构建

```powershell
npm install
npm run tauri build
```

安装包输出至 `src-tauri/target/release/bundle/nsis/`。

每次 `npm run tauri build` 自动递增补丁版本号，并同步更新
`package.json`、`package-lock.json`、`src-tauri/tauri.conf.json` 与
`src-tauri/Cargo.toml`：

```powershell
npm run tauri build            # 0.1.1 → 0.1.2 → 0.1.3 …
npm run tauri build -- minor   # 0.1.x → 0.2.0
npm run tauri build -- major   # 0.x.y → 1.0.0
```

`npm run tauri dev` 及其它 tauri 命令不会改动版本号。

## 工作原理

1. 依次检查 WebView2、Node.js 与 `dsh` CLI。
2. 若 `dsh` 缺失，通过阿里云 npm 镜像自动安装。
3. 查询并校验远端最新版本；发现新版时等待用户明确选择是否更新。
4. 执行 `dsh web --host 127.0.0.1 --port 0`，解析其输出的本地地址。
5. 验证 TCP 端口可连接后，主窗口跳转至 DSH 原始界面。
6. 应用退出时终止整个 DSH 进程树。

## 许可证

[MIT](./LICENSE) © 2026 wuxiaoji
