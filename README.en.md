# DeepSeek Harness Desktop

**English** · [简体中文](./README.md)

A lightweight Windows desktop host (Tauri 2) that launches the locally installed
**DeepSeek Harness** and displays its original Web UI. It never patches, copies,
or rebuilds DSH — it just runs it locally and hosts the window.

## ✨ Features

- 🚀 **One-click launch** — auto-checks WebView2 / Node.js / dsh and starts the DSH local service
- 📦 **Auto-install** — installs `dsh` automatically via npm (npmmirror mirror) when missing
- 🔄 **Update detection** — notifies you in-app when a new DSH version is available, one-click upgrade
- 🔒 **Local-first** — DSH listens only on `127.0.0.1` with an OS-assigned dynamic port
- 🧹 **Clean exit** — closing the window terminates the whole DSH process tree, no background residue
- 🐳 **Branding** — DeepSeek whale logo across the UI, desktop icon, taskbar, and installer

## 📋 Requirements

| Dependency | Notes |
| --- | --- |
| Windows 10/11 | Desktop-only for now |
| Microsoft Edge WebView2 Runtime | Bootstrapped by the installer when missing |
| Node.js | Must be on `PATH` (`node` / `npm`) |
| DeepSeek Harness (`dsh`) | Installed globally, `dsh --version` works |

Install DSH manually if needed:

```powershell
npm install --global @deepseek-ai/dsh
```

## 🛠️ Build from Source

```powershell
npm install
npm run tauri build
```

The release installer is generated at `src-tauri/target/release/bundle/nsis/`.

### Automatic Version Bump

Every `npm run tauri build` increments the **patch** version automatically
(synced across `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`,
and `src-tauri/Cargo.toml`):

```powershell
npm run tauri build            # 0.1.1 → 0.1.2 → 0.1.3 …
npm run tauri build -- minor   # 0.1.x → 0.2.0
npm run tauri build -- major   # 0.x.y → 1.0.0
```

> `npm run tauri dev` and other tauri commands never touch the version.

## ⚙️ How It Works

1. The launcher checks WebView2, Node.js, and the `dsh` CLI in sequence;
2. If `dsh` is missing, it is installed automatically via npm (npmmirror) and a marker is
   recorded so the uninstaller can clean it up safely;
3. Runs `dsh web --host 127.0.0.1 --port 0` and parses the local URL from its output;
4. After verifying the TCP port is reachable, the main window navigates to the DSH UI;
5. Closing the app terminates the entire DSH process tree.

## 📄 License

[MIT](./LICENSE)
