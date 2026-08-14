import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const phaseLabel = document.querySelector("#phase-label");
const statusTitle = document.querySelector("#status-title");
const statusDetail = document.querySelector("#status-detail");
const progressBar = document.querySelector("#progress-bar");
const retryButton = document.querySelector("#retry-button");
const updateBar = document.querySelector("#update-bar");
const updateMeta = document.querySelector("#update-meta");
const updateButton = document.querySelector("#update-button");
const skipButton = document.querySelector("#skip-button");
const confirmModal = document.querySelector("#update-confirm");
const confirmDetail = document.querySelector("#confirm-detail");
const confirmOk = document.querySelector("#confirm-ok");
const confirmCancel = document.querySelector("#confirm-cancel");
const rows = {
  webview: document.querySelector("#check-webview"),
  node: document.querySelector("#check-node"),
  dsh: document.querySelector("#check-dsh"),
  server: document.querySelector("#check-server"),
};

const SKIP_KEY = "dsh.skipUpdateVersion";
let navigating = false;
let pollTimer;
const stageProgress = { idle: 4, checking_webview: 14, checking_node: 30, checking_dsh: 52, installing_dsh: 60, updating_dsh: 64, starting_server: 72, waiting_for_server: 88, ready: 100, failed: 100, stopping: 100 };

function skippedVersion() {
  try { return localStorage.getItem(SKIP_KEY) || ""; } catch { return ""; }
}
function setSkippedVersion(version) {
  try { localStorage.setItem(SKIP_KEY, version || ""); } catch { /* ignore */ }
}

function setRow(row, state, value) {
  row.dataset.state = state;
  row.querySelector(".check-value").textContent = value;
}

function render(snapshot) {
  document.body.dataset.status = snapshot.status;
  phaseLabel.textContent = snapshot.phase_label;
  statusTitle.textContent = snapshot.title;
  statusDetail.textContent = snapshot.detail;
  progressBar.style.width = `${stageProgress[snapshot.step] ?? 8}%`;
  setRow(rows.webview, snapshot.webview2_available ? "ok" : "active", snapshot.webview2_available ? "可用" : "检测中");
  const nodeFailed = snapshot.failed_step === "checking_node";
  setRow(rows.node, snapshot.node_version ? "ok" : snapshot.step === "checking_node" ? "active" : nodeFailed ? "failed" : "waiting", snapshot.node_version ?? (snapshot.step === "checking_node" ? "检测中" : nodeFailed ? "未找到" : "等待"));
  const dshActive = ["checking_dsh", "installing_dsh", "updating_dsh"].includes(snapshot.step);
  const dshFailed = ["checking_dsh", "installing_dsh", "updating_dsh"].includes(snapshot.failed_step);
  setRow(rows.dsh, snapshot.dsh_version ? "ok" : dshActive ? "active" : dshFailed ? "failed" : "waiting", snapshot.dsh_version ?? (snapshot.step === "checking_dsh" ? "检测中" : snapshot.step === "installing_dsh" ? "安装中" : snapshot.step === "updating_dsh" ? "更新中" : snapshot.failed_step === "checking_dsh" ? "未找到" : snapshot.failed_step === "installing_dsh" ? "安装失败" : snapshot.failed_step === "updating_dsh" ? "更新失败" : "等待"));
  setRow(rows.server, snapshot.status === "ready" ? "ok" : ["starting_server", "waiting_for_server"].includes(snapshot.step) ? "active" : snapshot.status === "failed" ? "failed" : "waiting", snapshot.status === "ready" ? "已就绪" : snapshot.status === "failed" ? "未启动" : ["starting_server", "waiting_for_server"].includes(snapshot.step) ? "启动中" : "等待");

  const updating = snapshot.status === "updating";
  const latest = snapshot.dsh_latest || "";
  const skipped = latest && latest === skippedVersion();
  updateBar.hidden = !(snapshot.dsh_update_available || updating) || snapshot.status === "ready" || skipped;
  updateButton.disabled = updating;
  updateButton.textContent = updating ? "更新中…" : "立即更新";
  skipButton.hidden = updating;
  updateMeta.dataset.latest = latest;
  updateMeta.dataset.current = snapshot.dsh_version || "未知";
  updateMeta.textContent = updating ? "正在升级到最新版本…" : latest ? `当前 ${snapshot.dsh_version ?? "未知"} → 最新 ${latest}` : "";

  retryButton.hidden = snapshot.status !== "failed";
  if (snapshot.status === "ready" && snapshot.url && !navigating) {
    navigating = true;
    clearTimeout(pollTimer);
    statusDetail.textContent = `正在进入 ${snapshot.url}`;
    window.setTimeout(() => window.location.replace(snapshot.url), 320);
  }
}

async function pollStatus() {
  try {
    const snapshot = await invoke("launcher_status");
    render(snapshot);
    if (!navigating && snapshot.status !== "failed") pollTimer = window.setTimeout(pollStatus, 250);
  } catch (error) {
    document.body.dataset.status = "failed";
    phaseLabel.textContent = "LAUNCHER ERROR";
    statusTitle.textContent = "启动器通信失败";
    statusDetail.textContent = String(error);
    retryButton.hidden = false;
  }
}

async function launch() {
  retryButton.hidden = true;
  navigating = false;
  try {
    render(await invoke("launch_dsh"));
    clearTimeout(pollTimer);
    pollTimer = window.setTimeout(pollStatus, 150);
  } catch (error) {
    statusTitle.textContent = "无法开始启动";
    statusDetail.textContent = String(error);
    retryButton.hidden = false;
  }
}

function openConfirm() {
  const latest = updateMeta.dataset.latest || "最新版本";
  const current = updateMeta.dataset.current || "当前版本";
  confirmDetail.textContent = `确认将 DeepSeek Harness 从 ${current} 更新到 ${latest}？更新期间会临时关闭本地服务并重新下载，请确认后继续。`;
  confirmModal.hidden = false;
}

function closeConfirm() {
  confirmModal.hidden = true;
}

async function confirmUpdate() {
  closeConfirm();
  updateButton.disabled = true;
  updateButton.textContent = "更新中…";
  try {
    render(await invoke("update_dsh"));
  } catch (error) {
    statusTitle.textContent = "无法开始更新";
    statusDetail.textContent = String(error);
    updateButton.disabled = false;
    updateButton.textContent = "立即更新";
  }
}

retryButton.addEventListener("click", launch);
updateButton.addEventListener("click", openConfirm);
skipButton.addEventListener("click", () => {
  const latest = updateMeta.dataset.latest;
  if (latest) setSkippedVersion(latest);
  updateBar.hidden = true;
});
confirmCancel.addEventListener("click", closeConfirm);
confirmOk.addEventListener("click", confirmUpdate);
confirmModal.addEventListener("click", (event) => {
  if (event.target === confirmModal) closeConfirm();
});
window.addEventListener("DOMContentLoaded", launch);
