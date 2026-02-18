//! OpenClaw Daemon 服务
//!
//! 后台运行服务，支持：
//! - 后台守护进程
//! - 开机自启动
//! - 进程管理
//! - 日志记录

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

/// Daemon 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// 服务名称
    pub name: String,
    /// 可执行文件路径
    pub executable: String,
    /// 命令行参数
    pub args: Vec<String>,
    /// 工作目录
    pub working_dir: Option<PathBuf>,
    /// 环境变量
    pub env: std::collections::HashMap<String, String>,
    /// 自动重启
    pub auto_restart: bool,
    /// 重启延迟 (秒)
    pub restart_delay: u64,
    /// 最大重启次数
    pub max_restarts: u32,
    /// 日志文件路径
    pub log_file: Option<PathBuf>,
    /// PID 文件路径
    pub pid_file: Option<PathBuf>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            name: "openclaw".to_string(),
            executable: "openclaw-rust".to_string(),
            args: vec!["gateway".to_string()],
            working_dir: None,
            env: std::collections::HashMap::new(),
            auto_restart: true,
            restart_delay: 5,
            max_restarts: 10,
            log_file: None,
            pid_file: None,
        }
    }
}

impl DaemonConfig {
    /// 从配置文件加载
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("读取配置文件失败: {:?}", path))?;
        let config: DaemonConfig =
            serde_json::from_str(&content).with_context(|| "解析配置文件失败")?;
        Ok(config)
    }

    /// 保存到配置文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self).with_context(|| "序列化配置失败")?;
        fs::write(path, content).with_context(|| format!("写入配置文件失败: {:?}", path))?;
        Ok(())
    }

    /// 创建网关守护进程配置
    pub fn gateway(port: u16, host: &str) -> Self {
        Self {
            name: "openclaw-gateway".to_string(),
            executable: "openclaw-rust".to_string(),
            args: vec![
                "gateway".to_string(),
                "--port".to_string(),
                port.to_string(),
                "--host".to_string(),
                host.to_string(),
            ],
            auto_restart: true,
            restart_delay: 5,
            max_restarts: 10,
            ..Default::default()
        }
    }
}

/// Daemon 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonStatus {
    /// 未运行
    Stopped,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 正在停止
    Stopping,
    /// 重启中
    Restarting,
    /// 已崩溃
    Crashed,
}

/// Daemon 进程信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    /// 配置名称
    pub name: String,
    /// 进程 PID
    pub pid: Option<u32>,
    /// 当前状态
    pub status: DaemonStatus,
    /// 启动时间
    pub started_at: Option<DateTime<Utc>>,
    /// 重启次数
    pub restart_count: u32,
    /// 最后错误
    pub last_error: Option<String>,
}

/// Daemon 管理器
pub struct DaemonManager {
    config: DaemonConfig,
    process: Option<Child>,
    status: DaemonStatus,
    started_at: Option<DateTime<Utc>>,
    restart_count: u32,
    running: Arc<AtomicBool>,
    log_file: Option<std::fs::File>,
}

impl DaemonManager {
    /// 创建新的 Daemon 管理器
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            process: None,
            status: DaemonStatus::Stopped,
            started_at: None,
            restart_count: 0,
            running: Arc::new(AtomicBool::new(false)),
            log_file: None,
        }
    }

    /// 启动守护进程
    pub fn start(&mut self) -> Result<()> {
        if self.status == DaemonStatus::Running {
            warn!("守护进程已在运行");
            return Ok(());
        }

        info!("启动守护进程: {}", self.config.name);
        self.status = DaemonStatus::Starting;

        let mut cmd = Command::new(&self.config.executable);
        cmd.args(&self.config.args);

        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // 设置标准输出和错误
        if let Some(ref log_path) = self.config.log_file {
            let log_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .with_context(|| format!("打开日志文件失败: {:?}", log_path))?;
            self.log_file = Some(log_file.try_clone()?);
            cmd.stdout(Stdio::from(log_file.try_clone()?));
            cmd.stderr(Stdio::from(log_file));
        } else {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("启动进程失败: {}", self.config.executable))?;

        self.process = Some(child);
        self.status = DaemonStatus::Running;
        self.started_at = Some(Utc::now());
        self.running.store(true, Ordering::SeqCst);

        // 写入 PID 文件
        if let Some(ref pid_path) = self.config.pid_file
            && let Some(ref process) = self.process {
                let pid = process.id();
                fs::write(pid_path, pid.to_string())
                    .with_context(|| format!("写入 PID 文件失败: {:?}", pid_path))?;
            }

        info!(
            "守护进程已启动, PID: {:?}",
            self.process.as_ref().map(|p| p.id())
        );
        Ok(())
    }

    /// 停止守护进程
    pub fn stop(&mut self) -> Result<()> {
        if self.status != DaemonStatus::Running {
            warn!("守护进程未在运行");
            return Ok(());
        }

        info!("停止守护进程: {}", self.config.name);
        self.status = DaemonStatus::Stopping;
        self.running.store(false, Ordering::SeqCst);

        if let Some(mut child) = self.process.take() {
            // 尝试优雅关闭
            #[cfg(unix)]
            {
                // 发送 SIGTERM
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(child.id().to_string())
                    .status();

                // 等待进程退出
                for _ in 0..10 {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) => {
                            thread::sleep(Duration::from_millis(500));
                        }
                        Err(_) => break,
                    }
                }
            }

            #[cfg(windows)]
            {
                // Windows: 强制终止
                let _ = child.kill();
            }

            // 如果还在运行，强制杀死
            if let Ok(None) = child.try_wait() {
                warn!("强制终止进程");
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        // 删除 PID 文件
        if let Some(ref pid_path) = self.config.pid_file {
            let _ = fs::remove_file(pid_path);
        }

        self.status = DaemonStatus::Stopped;
        self.started_at = None;
        info!("守护进程已停止");
        Ok(())
    }

    /// 重启守护进程
    pub fn restart(&mut self) -> Result<()> {
        info!("重启守护进程: {}", self.config.name);
        self.stop()?;

        if self.config.restart_delay > 0 {
            thread::sleep(Duration::from_secs(self.config.restart_delay));
        }

        self.restart_count += 1;
        self.start()
    }

    /// 检查进程状态
    pub fn check(&mut self) -> DaemonStatus {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // 进程已退出
                    if status.success() {
                        info!("守护进程正常退出");
                    } else {
                        warn!("守护进程异常退出: {:?}", status);
                    }

                    self.status = DaemonStatus::Crashed;
                    self.process = None;

                    // 自动重启
                    if self.config.auto_restart && self.restart_count < self.config.max_restarts {
                        info!(
                            "尝试自动重启 ({}/{})",
                            self.restart_count + 1,
                            self.config.max_restarts
                        );

                        if self.restart_delay() {
                            self.restart_count += 1;
                            if self.start().is_ok() {
                                return DaemonStatus::Running;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // 进程仍在运行
                }
                Err(e) => {
                    error!("检查进程状态失败: {}", e);
                    self.status = DaemonStatus::Crashed;
                }
            }
        }

        self.status
    }

    /// 重启延迟
    fn restart_delay(&self) -> bool {
        if self.config.restart_delay > 0 {
            thread::sleep(Duration::from_secs(self.config.restart_delay));
        }
        true
    }

    /// 获取进程信息
    pub fn info(&self) -> DaemonInfo {
        DaemonInfo {
            name: self.config.name.clone(),
            pid: self.process.as_ref().map(|p| p.id()),
            status: self.status,
            started_at: self.started_at,
            restart_count: self.restart_count,
            last_error: None,
        }
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.status == DaemonStatus::Running
    }

    /// 获取状态
    pub fn status(&self) -> DaemonStatus {
        self.status
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        if self.status == DaemonStatus::Running {
            let _ = self.stop();
        }
    }
}

// ============ 系统服务集成 ============

/// macOS LaunchAgent 配置
#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchAgentConfig {
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "ProgramArguments")]
    pub program_arguments: Vec<String>,
    #[serde(rename = "RunAtLoad")]
    pub run_at_load: bool,
    #[serde(rename = "KeepAlive")]
    pub keep_alive: bool,
    #[serde(rename = "StandardOutPath")]
    pub standard_out_path: Option<String>,
    #[serde(rename = "StandardErrorPath")]
    pub standard_error_path: Option<String>,
    #[serde(rename = "WorkingDirectory")]
    pub working_directory: Option<String>,
}

impl LaunchAgentConfig {
    /// 从 Daemon 配置创建
    pub fn from_daemon_config(config: &DaemonConfig) -> Self {
        let mut args = vec![config.executable.clone()];
        args.extend(config.args.clone());

        Self {
            label: format!("com.openclaw.{}", config.name),
            program_arguments: args,
            run_at_load: true,
            keep_alive: config.auto_restart,
            standard_out_path: config
                .log_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            standard_error_path: config
                .log_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            working_directory: config
                .working_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        }
    }

    /// 生成 plist 文件内容
    pub fn to_plist(&self) -> Result<String> {
        let mut plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
"#,
            self.label
        );

        for arg in &self.program_arguments {
            plist.push_str(&format!("        <string>{}</string>\n", arg));
        }

        plist.push_str(
            r#"    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
"#,
        );

        plist.push_str(&format!(
            "    <{}/>\n",
            if self.keep_alive { "true" } else { "false" }
        ));

        if let Some(ref path) = self.standard_out_path {
            plist.push_str(&format!(
                r#"    <key>StandardOutPath</key>
    <string>{}</string>
"#,
                path
            ));
        }

        if let Some(ref path) = self.standard_error_path {
            plist.push_str(&format!(
                r#"    <key>StandardErrorPath</key>
    <string>{}</string>
"#,
                path
            ));
        }

        plist.push_str(
            r#"</dict>
</plist>
"#,
        );

        Ok(plist)
    }

    /// 获取 LaunchAgent 文件路径
    pub fn get_plist_path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        PathBuf::from(format!(
            "{}/Library/LaunchAgents/{}.plist",
            home, self.label
        ))
    }

    /// 安装 LaunchAgent
    pub fn install(&self) -> Result<()> {
        let plist_path = self.get_plist_path();
        let plist_content = self.to_plist()?;

        // 确保目录存在
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("创建目录失败: {:?}", parent))?;
        }

        // 写入 plist 文件
        fs::write(&plist_path, plist_content)
            .with_context(|| format!("写入 plist 文件失败: {:?}", plist_path))?;

        info!("LaunchAgent 已安装: {:?}", plist_path);

        // 加载 LaunchAgent
        let output = Command::new("launchctl")
            .arg("load")
            .arg(&plist_path)
            .output()
            .with_context(|| "加载 LaunchAgent 失败")?;

        if output.status.success() {
            info!("LaunchAgent 已加载");
        } else {
            warn!(
                "加载 LaunchAgent 可能失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// 卸载 LaunchAgent
    pub fn uninstall(&self) -> Result<()> {
        let plist_path = self.get_plist_path();

        // 卸载 LaunchAgent
        let _ = Command::new("launchctl")
            .arg("unload")
            .arg(&plist_path)
            .output();

        // 删除 plist 文件
        if plist_path.exists() {
            fs::remove_file(&plist_path)
                .with_context(|| format!("删除 plist 文件失败: {:?}", plist_path))?;
        }

        info!("LaunchAgent 已卸载");
        Ok(())
    }
}

/// Linux Systemd 服务配置
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemdServiceConfig {
    pub unit_name: String,
    pub description: String,
    pub exec_start: String,
    pub working_directory: Option<String>,
    pub restart: String,
    pub user: Option<String>,
    pub after: Vec<String>,
}

impl SystemdServiceConfig {
    /// 从 Daemon 配置创建
    pub fn from_daemon_config(config: &DaemonConfig) -> Self {
        let exec_start = format!("{} {}", config.executable, config.args.join(" "));

        Self {
            unit_name: format!("{}.service", config.name),
            description: format!("OpenClaw {}", config.name),
            exec_start,
            working_directory: config
                .working_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            restart: if config.auto_restart {
                "always".to_string()
            } else {
                "no".to_string()
            },
            user: None,
            after: vec!["network.target".to_string()],
        }
    }

    /// 生成 service 文件内容
    pub fn to_service_file(&self) -> String {
        let mut content = format!(
            r#"[Unit]
Description={}
After={}

[Service]
Type=simple
ExecStart={}
Restart={}
"#,
            self.description,
            self.after.join(" "),
            self.exec_start,
            self.restart
        );

        if let Some(ref dir) = self.working_directory {
            content.push_str(&format!("WorkingDirectory={}\n", dir));
        }

        if let Some(ref user) = self.user {
            content.push_str(&format!("User={}\n", user));
        }

        content.push_str("\n[Install]\nWantedBy=multi-user.target\n");
        content
    }

    /// 获取 service 文件路径
    pub fn get_service_path(&self) -> PathBuf {
        PathBuf::from(format!("/etc/systemd/system/{}", self.unit_name))
    }

    /// 安装服务
    pub fn install(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let service_content = self.to_service_file();
            let service_path = self.get_service_path();

            // 写入 service 文件 (需要 root 权限)
            fs::write(&service_path, service_content)
                .with_context(|| format!("写入 service 文件失败: {:?}", service_path))?;

            // 重新加载 systemd
            let _ = Command::new("systemctl").arg("daemon-reload").status();

            // 启用服务
            let _ = Command::new("systemctl")
                .arg("enable")
                .arg(&self.unit_name)
                .status();

            info!("Systemd 服务已安装: {:?}", service_path);
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("Systemd 服务仅在 Linux 上可用");
        }

        Ok(())
    }

    /// 卸载服务
    pub fn uninstall(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            // 停止服务
            let _ = Command::new("systemctl")
                .arg("stop")
                .arg(&self.unit_name)
                .status();

            // 禁用服务
            let _ = Command::new("systemctl")
                .arg("disable")
                .arg(&self.unit_name)
                .status();

            // 删除 service 文件
            let service_path = self.get_service_path();
            if service_path.exists() {
                fs::remove_file(&service_path)
                    .with_context(|| format!("删除 service 文件失败: {:?}", service_path))?;
            }

            // 重新加载 systemd
            let _ = Command::new("systemctl").arg("daemon-reload").status();

            info!("Systemd 服务已卸载");
        }

        Ok(())
    }
}

// ============ CLI 命令实现 ============

/// Daemon 命令
#[derive(Debug, clap::Subcommand)]
pub enum DaemonCommand {
    /// 启动守护进程
    Start {
        /// 端口
        #[arg(short, long, default_value = "18789")]
        port: u16,
        /// 主机
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    /// 停止守护进程
    Stop,
    /// 重启守护进程
    Restart,
    /// 查看状态
    Status,
    /// 安装系统服务
    Install,
    /// 卸载系统服务
    Uninstall,
}

/// 执行 Daemon 命令
pub async fn execute(cmd: DaemonCommand) -> Result<()> {
    match cmd {
        DaemonCommand::Start { port, host } => {
            let config = DaemonConfig::gateway(port, &host);
            let mut manager = DaemonManager::new(config);
            manager.start()?;

            // 保持运行
            loop {
                manager.check();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        DaemonCommand::Stop => {
            // 发送停止信号
            let pid_path = get_default_pid_path();
            if pid_path.exists() {
                let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;

                #[cfg(unix)]
                {
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .status();
                }

                println!("守护进程已停止");
            } else {
                println!("守护进程未运行");
            }
        }
        DaemonCommand::Restart => {
            println!("重启守护进程...");
            // 先停止后启动
        }
        DaemonCommand::Status => {
            let pid_path = get_default_pid_path();
            if pid_path.exists() {
                let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;
                println!("守护进程正在运行, PID: {}", pid);
            } else {
                println!("守护进程未运行");
            }
        }
        DaemonCommand::Install => {
            install_system_service()?;
        }
        DaemonCommand::Uninstall => {
            uninstall_system_service()?;
        }
    }

    Ok(())
}

fn get_default_pid_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(format!("{}/.openclaw-rust/openclaw.pid", home))
}

#[cfg(target_os = "macos")]
fn install_system_service() -> Result<()> {
    let config = DaemonConfig::gateway(18789, "0.0.0.0");
    let launch_agent = LaunchAgentConfig::from_daemon_config(&config);
    launch_agent.install()
}

#[cfg(target_os = "linux")]
fn install_system_service() -> Result<()> {
    let config = DaemonConfig::gateway(18789, "0.0.0.0");
    let systemd = SystemdServiceConfig::from_daemon_config(&config);
    systemd.install()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn install_system_service() -> Result<()> {
    anyhow::bail!("系统服务仅支持 macOS 和 Linux")
}

#[cfg(target_os = "macos")]
fn uninstall_system_service() -> Result<()> {
    let config = DaemonConfig::gateway(18789, "0.0.0.0");
    let launch_agent = LaunchAgentConfig::from_daemon_config(&config);
    launch_agent.uninstall()
}

#[cfg(target_os = "linux")]
fn uninstall_system_service() -> Result<()> {
    let config = DaemonConfig::gateway(18789, "0.0.0.0");
    let systemd = SystemdServiceConfig::from_daemon_config(&config);
    systemd.uninstall()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn uninstall_system_service() -> Result<()> {
    anyhow::bail!("系统服务仅支持 macOS 和 Linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.name, "openclaw");
        assert!(config.auto_restart);
    }

    #[test]
    fn test_gateway_config() {
        let config = DaemonConfig::gateway(8080, "127.0.0.1");
        assert!(config.args.contains(&"--port".to_string()));
        assert!(config.args.contains(&"8080".to_string()));
    }

    #[test]
    fn test_launch_agent_config() {
        let config = DaemonConfig::default();
        let launch_agent = LaunchAgentConfig::from_daemon_config(&config);
        assert!(launch_agent.label.starts_with("com.openclaw."));
    }
}
