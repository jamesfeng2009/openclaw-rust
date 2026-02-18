use crate::nodes::{DeviceError, SystemCommandResult};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SystemManager {
    allowed_commands: Arc<RwLock<Vec<String>>>,
}

impl SystemManager {
    pub fn new() -> Self {
        let allowed = vec![
            "echo".to_string(),
            "date".to_string(),
            "whoami".to_string(),
            "pwd".to_string(),
            "ls".to_string(),
            "cat".to_string(),
            "mkdir".to_string(),
            "touch".to_string(),
            "rm".to_string(),
            "cp".to_string(),
            "mv".to_string(),
        ];

        Self {
            allowed_commands: Arc::new(RwLock::new(allowed)),
        }
    }

    pub async fn run_command(
        &self,
        command: &str,
        args: Vec<String>,
    ) -> Result<SystemCommandResult, DeviceError> {
        let allowed = self.allowed_commands.read().await;

        if !allowed.contains(&command.to_string()) {
            return Ok(SystemCommandResult {
                success: false,
                stdout: None,
                stderr: None,
                exit_code: None,
                error: Some(format!("命令 '{}' 未被允许", command)),
            });
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new(command)
                .args(&args)
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            Ok(SystemCommandResult {
                success: output.status.success(),
                stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                exit_code: output.status.code(),
                error: None,
            })
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new(command)
                .args(&args)
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            Ok(SystemCommandResult {
                success: output.status.success(),
                stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                exit_code: output.status.code(),
                error: None,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new(command)
                .args(&args)
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            Ok(SystemCommandResult {
                success: output.status.success(),
                stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                exit_code: output.status.code(),
                error: None,
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Ok(SystemCommandResult {
                success: false,
                stdout: None,
                stderr: None,
                exit_code: None,
                error: Some("不支持的平台".to_string()),
            })
        }
    }

    pub async fn add_allowed_command(&self, command: String) {
        let mut allowed = self.allowed_commands.write().await;
        if !allowed.contains(&command) {
            allowed.push(command);
        }
    }

    pub async fn remove_allowed_command(&self, command: &str) {
        let mut allowed = self.allowed_commands.write().await;
        allowed.retain(|c| c != command);
    }

    pub async fn list_allowed_commands(&self) -> Vec<String> {
        self.allowed_commands.read().await.clone()
    }
}

impl Default for SystemManager {
    fn default() -> Self {
        Self::new()
    }
}
