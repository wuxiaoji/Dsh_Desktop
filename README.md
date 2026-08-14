# DeepSeek Harness 桌面版

[English](./README.en.md) · **简体中文**

一个轻量的 Windows 桌面主机（Tauri 2），用于启动本机安装的 **DeepSeek Harness** 并直接展示其原始 Web 界面。它不修改、不复制、不重新构建 DSH，只负责把 DSH 在本地跑起来并接管窗口。

## ✨ 特性

- 🚀 **一键启动** —— 自动检测 WebView2 / Node.js / dsh 运行环境，启动 DSH 本地服务
- 📦 **自动安装** —— 未检测到 dsh 时自动通过 npm（国内镜像源 npmmirror）安装，无需手动干预
- 🔄 **版本检测与升级** —— 发现 DSH 新版本时在界面内提示，一键更新到最新版
- 🔒 **本地优先** —— DSH 服务仅监听 `127.0.0.1`，端口由系统动态分配，不暴露到外网
- 🧹 **干净退出** —— 关闭窗口即终止整个 DSH 进程树，不留后台残留进程
- 🐳 **品牌标识** —— 内置 DeepSeek 鲸鱼 LOGO，覆盖应用界面、桌面图标、任务栏与安装包

## 📋 运行环境要求

| 依赖 | 说明 |
| --- | --- |
| Windows 10/11 | 当前仅支持 Windows 桌面 |
| Microsoft Edge WebView2 Runtime | 缺失时安装包会自动引导下载 |
| Node.js | 需加入 `PATH`，提供 `node` / `npm` 命令 |
| DeepSeek Harness (`dsh`) | 需全局安装，`dsh --version` 可用 |

手动安装 DSH：

```powershell
npm install --global @deepseek-ai/dsh
```

## 🛠️ 从源码构建

```powershell
npm install
npm run tauri build
```

发布安装包输出到 `src-tauri/target/release/bundle/nsis/`。

### 版本号自动递增

每次执行 `npm run tauri build` 都会自动递增**补丁号**，并同步更新
`package.json`、`package-lock.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`：

```powershell
npm run tauri build            # 0.1.1 → 0.1.2 → 0.1.3 …
npm run tauri build -- minor   # 0.1.x → 0.2.0
npm run tauri build -- major   # 0.x.y → 1.0.0
```

> `npm run tauri dev` 及其它 tauri 命令不会改动版本号。

## ⚙️ 工作原理

1. 启动器依次检查 WebView2、Node.js、`dsh` CLI；
2. 若 `dsh` 缺失，自动通过 npm（npmmirror 镜像源）安装，并记录安装标记供卸载器清理；
3. 执行 `dsh web --host 127.0.0.1 --port 0`，从输出中解析 DSH 的本地访问地址；
4. 验证对应 TCP 端口可连接后，主窗口跳转到 DSH 原始界面；
5. 关闭应用时终止整个 DSH 进程树，服务随窗口退出。

## 📄 许可证

[MIT](./LICENSE)
