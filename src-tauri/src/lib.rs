use serde::Serialize;
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{Manager, State};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_LOG_LINES: usize = 120;
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const NPM_REGISTRY: &str = "https://registry.npmmirror.com";
const INSTALLED_MARKER: &str = ".dsh-installed-by-app";

#[derive(Clone, Serialize)]
struct LauncherSnapshot {
    status: String,
    step: String,
    phase_label: String,
    title: String,
    detail: String,
    webview2_available: bool,
    node_version: Option<String>,
    dsh_version: Option<String>,
    dsh_latest: Option<String>,
    dsh_update_available: bool,
    url: Option<String>,
    failed_step: Option<String>,
    logs: Vec<String>,
}

impl Default for LauncherSnapshot {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            step: "idle".into(),
            phase_label: "INITIALIZING".into(),
            title: "正在准备启动环境".into(),
            detail: "启动器正在连接本机运行时。".into(),
            webview2_available: false,
            node_version: None,
            dsh_version: None,
            dsh_latest: None,
            dsh_update_available: false,
            url: None,
            failed_step: None,
            logs: Vec::new(),
        }
    }
}

struct LauncherInner {
    snapshot: LauncherSnapshot,
    child_pid: Option<u32>,
    generation: u64,
}

struct LauncherState {
    inner: Arc<Mutex<LauncherInner>>,
}

impl LauncherState {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LauncherInner {
                snapshot: LauncherSnapshot::default(),
                child_pid: None,
                generation: 0,
            })),
        }
    }

    fn stop(&self) {
        let pid = {
            let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
            inner.snapshot.status = "stopping".into();
            inner.snapshot.step = "stopping".into();
            inner.child_pid.take()
        };
        if let Some(pid) = pid {
            kill_process_tree(pid);
        }
    }
}

#[tauri::command]
fn launcher_status(state: State<'_, LauncherState>) -> LauncherSnapshot {
    state
        .inner
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .snapshot
        .clone()
}

#[tauri::command]
fn launch_dsh(state: State<'_, LauncherState>) -> LauncherSnapshot {
    let (generation, snapshot) = {
        let mut inner = state.inner.lock().unwrap_or_else(|error| error.into_inner());
        if matches!(inner.snapshot.status.as_str(), "starting" | "ready") {
            return inner.snapshot.clone();
        }
        inner.generation += 1;
        inner.child_pid = None;
        inner.snapshot = LauncherSnapshot {
            status: "starting".into(),
            step: "checking_webview".into(),
            phase_label: "ENVIRONMENT 01/04".into(),
            title: "正在检查桌面运行时".into(),
            detail: "应用已由 WebView2 承载，继续检查本机 Node.js。".into(),
            webview2_available: true,
            ..LauncherSnapshot::default()
        };
        (inner.generation, inner.snapshot.clone())
    };

    let shared = Arc::clone(&state.inner);
    thread::spawn(move || launch_worker(shared, generation));
    snapshot
}

#[tauri::command]
fn update_dsh(state: State<'_, LauncherState>) -> LauncherSnapshot {
    let (generation, old_pid, snapshot) = {
        let mut inner = state.inner.lock().unwrap_or_else(|error| error.into_inner());
        if inner.snapshot.status == "updating" {
            return inner.snapshot.clone();
        }
        let old_pid = inner.child_pid.take();
        inner.generation += 1;
        inner.snapshot = LauncherSnapshot {
            status: "updating".into(),
            step: "updating_dsh".into(),
            phase_label: "UPDATE DSH".into(),
            title: "正在更新 DeepSeek Harness".into(),
            detail: format!("正在通过 npm 升级到最新版本（镜像源 {NPM_REGISTRY}）。"),
            webview2_available: true,
            node_version: inner.snapshot.node_version.clone(),
            dsh_version: inner.snapshot.dsh_version.clone(),
            ..LauncherSnapshot::default()
        };
        (inner.generation, old_pid, inner.snapshot.clone())
    };

    if let Some(pid) = old_pid {
        kill_process_tree(pid);
    }

    let shared = Arc::clone(&state.inner);
    thread::spawn(move || update_worker(shared, generation));
    snapshot
}

fn launch_worker(shared: Arc<Mutex<LauncherInner>>, generation: u64) {
    update(&shared, generation, |snapshot| {
        snapshot.step = "checking_node".into();
        snapshot.phase_label = "ENVIRONMENT 02/04".into();
        snapshot.title = "正在检查 Node.js".into();
        snapshot.detail = "通过 PATH 解析本机 Node.js 运行时。".into();
    });

    let node_version = match run_version("node --version") {
        Ok(version) => version,
        Err(error) => {
            fail(
                &shared,
                generation,
                "未找到 Node.js",
                format!("{error} 请安装 Node.js 并确认 node 命令已加入 PATH。"),
            );
            return;
        }
    };
    update(&shared, generation, |snapshot| snapshot.node_version = Some(node_version));

    update(&shared, generation, |snapshot| {
        snapshot.step = "checking_dsh".into();
        snapshot.phase_label = "ENVIRONMENT 03/04".into();
        snapshot.title = "正在检查 DeepSeek Harness".into();
        snapshot.detail = "验证 dsh 命令及其本地安装。".into();
    });

    let dsh_version = match run_version("dsh --version") {
        Ok(version) => version,
        Err(_) => match install_dsh(&shared, generation) {
            Ok(version) => version,
            Err(error) => {
                if current_status(&shared, generation) != "stopping" {
                    fail(
                        &shared,
                        generation,
                        "DeepSeek Harness 安装失败",
                        format!("{error} 也可手动执行 npm install --global @deepseek-ai/dsh --registry={NPM_REGISTRY} 后重试。"),
                    );
                }
                return;
            }
        },
    };
    update(&shared, generation, |snapshot| snapshot.dsh_version = Some(dsh_version.clone()));
    check_dsh_update(&shared, generation, &dsh_version);

    if let Err(error) = start_dsh_server(&shared, generation) {
        if current_status(&shared, generation) != "stopping" {
            fail(&shared, generation, "DSH 启动失败", error);
        }
    }
}

fn update_worker(shared: Arc<Mutex<LauncherInner>>, generation: u64) {
    if let Err(error) = run_version("npm --version") {
        if current_status(&shared, generation) != "stopping" {
            fail(
                &shared,
                generation,
                "DSH 更新失败",
                format!("未找到 npm：{error} 请安装 Node.js（自带 npm）后重试。"),
            );
        }
        return;
    }

    let install_command = format!("npm install --global @deepseek-ai/dsh@latest --registry={NPM_REGISTRY}");
    if let Err(error) = run_npm_command(&shared, generation, &install_command) {
        if current_status(&shared, generation) != "stopping" {
            fail(&shared, generation, "DSH 更新失败", error);
        }
        return;
    }
    write_installed_marker();

    let dsh_version = match run_version("dsh --version") {
        Ok(version) => version,
        Err(error) => {
            if current_status(&shared, generation) != "stopping" {
                fail(
                    &shared,
                    generation,
                    "DSH 更新失败",
                    format!("安装完成但 dsh 命令不可用：{error}"),
                );
            }
            return;
        }
    };
    update(&shared, generation, |snapshot| {
        snapshot.dsh_version = Some(dsh_version);
        snapshot.dsh_latest = None;
        snapshot.dsh_update_available = false;
    });

    if let Err(error) = start_dsh_server(&shared, generation) {
        if current_status(&shared, generation) != "stopping" {
            fail(&shared, generation, "DSH 启动失败", error);
        }
    }
}

fn spawn_dsh() -> Result<Child, String> {
    let mut command = shell_command("dsh web --host 127.0.0.1 --port 0");
    command.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    if let Some(home) = std::env::var_os("USERPROFILE") {
        command.current_dir(home);
    }
    command
        .spawn()
        .map_err(|error| format!("无法创建 dsh 子进程：{error}"))
}

fn monitor_dsh(shared: &Arc<Mutex<LauncherInner>>, generation: u64, child: &mut Child) {
    let (sender, receiver) = mpsc::channel::<(bool, String)>();
    if let Some(stdout) = child.stdout.take() {
        let sender = sender.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = sender.send((true, line));
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sender = sender.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = sender.send((false, line));
            }
        });
    }
    drop(sender);

    let started = Instant::now();
    let mut ready = false;

    loop {
        if let Ok((is_stdout, line)) = receiver.recv_timeout(Duration::from_millis(200)) {
            push_log(shared, generation, if is_stdout { "OUT" } else { "ERR" }, &line);
            if !ready {
                if let Some(url) = extract_local_url(&line) {
                    update(shared, generation, |snapshot| {
                        snapshot.step = "waiting_for_server".into();
                        snapshot.title = "正在确认服务就绪".into();
                        snapshot.detail = format!("等待 {url} 接受本地连接。");
                    });
                    match wait_for_server(&url, CONNECT_TIMEOUT) {
                        Ok(()) => {
                            ready = true;
                            update(shared, generation, |snapshot| {
                                snapshot.status = "ready".into();
                                snapshot.step = "ready".into();
                                snapshot.phase_label = "READY".into();
                                snapshot.title = "DeepSeek Harness 已就绪".into();
                                snapshot.detail = "正在切换到 DSH 原始界面。".into();
                                snapshot.url = Some(url);
                            });
                        }
                        Err(error) => {
                            kill_process_tree(child.id());
                            fail(shared, generation, "本地服务未就绪", error);
                            return;
                        }
                    }
                }
            }
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                clear_pid(shared, generation);
                let stopping = current_status(shared, generation) == "stopping";
                if !stopping {
                    fail(
                        shared,
                        generation,
                        "DSH 已退出",
                        format!("本地服务进程提前结束，退出状态：{status}。"),
                    );
                }
                return;
            }
            Ok(None) => {}
            Err(error) => {
                fail(shared, generation, "无法监控 DSH", error.to_string());
                return;
            }
        }

        if !ready && started.elapsed() >= STARTUP_TIMEOUT {
            kill_process_tree(child.id());
            fail(
                shared,
                generation,
                "DSH 启动超时",
                "60 秒内没有收到 DSH 的本地访问地址，请检查启动日志和本机配置。".into(),
            );
            return;
        }
    }
}

fn run_version(command_line: &str) -> Result<String, String> {
    let output = shell_command(command_line)
        .output()
        .map_err(|error| format!("无法执行 {command_line}：{error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            format!("命令 {command_line} 返回 {}。", output.status)
        } else {
            detail
        });
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Err(format!("命令 {command_line} 没有返回版本信息。"))
    } else {
        Ok(value.lines().next().unwrap_or_default().trim().to_string())
    }
}

fn install_dsh(shared: &Arc<Mutex<LauncherInner>>, generation: u64) -> Result<String, String> {
    update(shared, generation, |snapshot| {
        snapshot.step = "installing_dsh".into();
        snapshot.phase_label = "INSTALL 03/04".into();
        snapshot.title = "正在安装 DeepSeek Harness".into();
        snapshot.detail = format!("未检测到 dsh，正在通过 npm 自动安装（镜像源 {NPM_REGISTRY}）。");
    });

    if let Err(error) = run_version("npm --version") {
        return Err(format!("未找到 npm：{error} 请安装 Node.js（自带 npm）后重试。"));
    }

    update(shared, generation, |snapshot| {
        snapshot.detail = "npm 正在下载并安装 @deepseek-ai/dsh，可能需要几分钟，请稍候。".into();
    });

    let install_command = format!("npm install --global @deepseek-ai/dsh --registry={NPM_REGISTRY}");
    run_npm_command(shared, generation, &install_command)?;

    // 记录“由本应用安装”标记：卸载器据此决定是否清理全局 dsh，避免误删用户独立安装的 dsh
    write_installed_marker();

    match run_version("dsh --version") {
        Ok(version) => Ok(version),
        Err(error) => Err(format!("安装已完成，但 dsh 命令仍不可用：{error}")),
    }
}

fn run_npm_command(shared: &Arc<Mutex<LauncherInner>>, generation: u64, command_line: &str) -> Result<(), String> {
    let mut command = shell_command(command_line);
    command.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    if let Some(home) = std::env::var_os("USERPROFILE") {
        command.current_dir(home);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("无法创建 npm 子进程：{error}"))?;

    {
        let mut inner = shared.lock().unwrap_or_else(|error| error.into_inner());
        if inner.generation != generation {
            let _ = child.kill();
            return Err("启动已取消。".into());
        }
        inner.child_pid = Some(child.id());
    }

    let (sender, receiver) = mpsc::channel::<String>();
    if let Some(stdout) = child.stdout.take() {
        let sender = sender.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sender = sender.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });
    }
    drop(sender);

    let started = Instant::now();
    loop {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(200)) {
            push_log(shared, generation, "NPM", &line);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                clear_pid(shared, generation);
                if status.success() {
                    return Ok(());
                }
                return Err(format!("npm 安装失败，退出状态：{status}。"));
            }
            Ok(None) => {}
            Err(error) => {
                clear_pid(shared, generation);
                return Err(format!("无法监控 npm 安装进程：{error}"));
            }
        }
        if started.elapsed() >= INSTALL_TIMEOUT {
            kill_process_tree(child.id());
            clear_pid(shared, generation);
            return Err("npm 安装超时（10 分钟），请检查网络后重试。".into());
        }
    }
}

fn write_installed_marker() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::fs::write(dir.join(INSTALLED_MARKER), "1");
        }
    }
}

fn start_dsh_server(shared: &Arc<Mutex<LauncherInner>>, generation: u64) -> Result<(), String> {
    update(shared, generation, |snapshot| {
        snapshot.step = "starting_server".into();
        snapshot.phase_label = "SERVICE 04/04".into();
        snapshot.title = "正在启动本地服务".into();
        snapshot.detail = "DSH 将仅监听 127.0.0.1，并由系统分配空闲端口。".into();
    });

    let mut child = spawn_dsh()?;

    let pid = child.id();
    {
        let mut inner = shared.lock().unwrap_or_else(|error| error.into_inner());
        if inner.generation != generation {
            let _ = child.kill();
            return Err("启动已取消。".into());
        }
        inner.child_pid = Some(pid);
    }

    monitor_dsh(shared, generation, &mut child);
    Ok(())
}

fn check_dsh_update(shared: &Arc<Mutex<LauncherInner>>, generation: u64, current: &str) {
    let current = normalize_version(current);
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        let latest = match run_version(&format!(
            "npm view @deepseek-ai/dsh version --registry={NPM_REGISTRY}"
        )) {
            Ok(version) => normalize_version(&version),
            Err(_) => return, // 查询失败（如离线）则静默跳过，不阻塞启动
        };
        if !latest.is_empty() && latest != current {
            update(&shared, generation, |snapshot| {
                snapshot.dsh_latest = Some(latest);
                snapshot.dsh_update_available = true;
            });
        }
    });
}

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .split_whitespace()
        .find(|part| part.chars().next().map_or(false, |c| c.is_ascii_digit()) || part.starts_with('v'))
        .unwrap_or(value.trim())
        .trim_start_matches(['v', 'V'])
        .trim_end_matches(['.', '-'])
        .to_string()
}

fn shell_command(command_line: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/S", "/C", command_line]);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut command = Command::new("sh");
        command.args(["-lc", command_line]);
        command
    }
}

fn extract_local_url(line: &str) -> Option<String> {
    let start = line.find("dsh web: http://127.0.0.1:")? + "dsh web: ".len();
    let candidate = line[start..].split_whitespace().next()?.trim_end_matches('/');
    let port = candidate.rsplit(':').next()?;
    if port.parse::<u16>().is_ok() {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn wait_for_server(url: &str, timeout: Duration) -> Result<(), String> {
    let port = url
        .rsplit(':')
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| format!("DSH 返回了无法识别的地址：{url}"))?;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let started = Instant::now();
    while started.elapsed() < timeout {
        if TcpStream::connect_timeout(&address, Duration::from_millis(350)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(180));
    }
    Err(format!("无法连接 DSH 本地端口 {port}。"))
}

fn update<F>(shared: &Arc<Mutex<LauncherInner>>, generation: u64, change: F)
where
    F: FnOnce(&mut LauncherSnapshot),
{
    let mut inner = shared.lock().unwrap_or_else(|error| error.into_inner());
    if inner.generation == generation {
        change(&mut inner.snapshot);
    }
}

fn fail(shared: &Arc<Mutex<LauncherInner>>, generation: u64, title: &str, detail: String) {
    update(shared, generation, |snapshot| {
        snapshot.failed_step = Some(snapshot.step.clone());
        snapshot.status = "failed".into();
        snapshot.step = "failed".into();
        snapshot.phase_label = "STARTUP FAILED".into();
        snapshot.title = title.into();
        snapshot.detail = detail;
        snapshot.url = None;
    });
    clear_pid(shared, generation);
}

fn push_log(shared: &Arc<Mutex<LauncherInner>>, generation: u64, stream: &str, line: &str) {
    let mut inner = shared.lock().unwrap_or_else(|error| error.into_inner());
    if inner.generation != generation {
        return;
    }
    let mut logs: VecDeque<String> = inner.snapshot.logs.drain(..).collect();
    logs.push_back(format!("[{stream}] {line}"));
    while logs.len() > MAX_LOG_LINES {
        logs.pop_front();
    }
    inner.snapshot.logs = logs.into_iter().collect();
}

fn clear_pid(shared: &Arc<Mutex<LauncherInner>>, generation: u64) {
    let mut inner = shared.lock().unwrap_or_else(|error| error.into_inner());
    if inner.generation == generation {
        inner.child_pid = None;
    }
}

fn current_status(shared: &Arc<Mutex<LauncherInner>>, generation: u64) -> String {
    let inner = shared.lock().unwrap_or_else(|error| error.into_inner());
    if inner.generation == generation {
        inner.snapshot.status.clone()
    } else {
        "stopping".into()
    }
}

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("taskkill.exe");
        command.args(["/PID", &pid.to_string(), "/T", "/F"]);
        command.creation_flags(CREATE_NO_WINDOW);
        let _ = command.output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        .manage(LauncherState::new())
        .invoke_handler(tauri::generate_handler![launch_dsh, launcher_status, update_dsh])
        .build(tauri::generate_context!())
        .expect("failed to build the DSH desktop host");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
            app_handle.state::<LauncherState>().stop();
        }
    });
}
