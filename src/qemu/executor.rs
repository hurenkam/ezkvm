//! QEMU process execution
//!
//! Handles spawning and managing QEMU processes.

use super::types::QemuArgs;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use anyhow::{anyhow, Result};
use nix::unistd::Pid;
use nix::sys::signal::{kill, Signal};

/// QEMU process executor
pub struct QemuExecutor {
    binary: String,
    args: QemuArgs,
}

impl QemuExecutor {
    /// Create a new executor
    pub fn new(binary: String, args: QemuArgs) -> Self {
        Self { binary, args }
    }
    
    /// Execute QEMU synchronously (blocking)
    pub fn execute_sync(&self) -> Result<std::process::ExitStatus> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(self.args.as_ref());
        
        // Inherit stdin/stdout/stderr for interactive use
        cmd.stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());
        
        let status = cmd.status()
            .map_err(|e| anyhow!("Failed to execute QEMU: {}", e))?;
        
        Ok(status)
    }

    /// Execute QEMU synchronously while redirecting stdout/stderr to a log file.
    pub fn execute_sync_logged(&self, log_file: &Path, stdin: Stdio) -> Result<std::process::ExitStatus> {
        let stdout_file = File::options()
            .create(true)
            .append(true)
            .open(log_file)
            .map_err(|e| anyhow!("Failed to open log file '{}': {}", log_file.display(), e))?;
        let stderr_file = stdout_file
            .try_clone()
            .map_err(|e| anyhow!("Failed to clone log file handle '{}': {}", log_file.display(), e))?;

        let mut cmd = Command::new(&self.binary);
        cmd.args(self.args.as_ref());
        cmd.stdin(stdin)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file));

        let status = cmd.status()
            .map_err(|e| anyhow!("Failed to execute QEMU: {}", e))?;

        Ok(status)
    }
    
    /// Execute QEMU asynchronously (background)
    pub async fn execute_async(&self) -> Result<QemuProcess> {
        let mut cmd = TokioCommand::new(&self.binary);
        cmd.args(self.args.as_ref());
        
        // For background processes, we might want to redirect output
        cmd.stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn QEMU: {}", e))?;
        
        let pid = child.id()
            .ok_or_else(|| anyhow!("Failed to get process ID"))?;
        
        Ok(QemuProcess {
            pid: pid as i32,
            child: Some(child),
        })
    }
    
    /// Dry run - just print the command that would be executed
    pub fn dry_run(&self) -> String {
        let mut cmd = vec![self.binary.clone()];
        cmd.extend(self.args.clone().into_inner());
        cmd.join(" ")
    }
}

/// Handle to a running QEMU process
pub struct QemuProcess {
    pid: i32,
    child: Option<tokio::process::Child>,
}

impl QemuProcess {
    /// Get the process ID
    pub fn pid(&self) -> i32 {
        self.pid
    }
    
    /// Wait for the process to complete
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(child) = &mut self.child {
            let status = child.wait().await
                .map_err(|e| anyhow!("Failed to wait for QEMU process: {}", e))?;
            Ok(status)
        } else {
            Err(anyhow!("Process already finished"))
        }
    }
    
    /// Send a signal to the process
    pub fn signal(&self, signal: Signal) -> Result<()> {
        kill(Pid::from_raw(self.pid), signal)
            .map_err(|e| anyhow!("Failed to send signal to QEMU process: {}", e))
    }
    
    /// Terminate the process gracefully
    pub fn terminate(&self) -> Result<()> {
        self.signal(Signal::SIGTERM)
    }
    
    /// Kill the process forcefully
    pub fn kill(&self) -> Result<()> {
        self.signal(Signal::SIGKILL)
    }
    
    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}

impl Drop for QemuProcess {
    fn drop(&mut self) {
        // Try to terminate the process when the handle is dropped
        if self.is_running() {
            let _ = self.terminate();
        }
    }
}

/// Check if QEMU binary is available
pub fn check_qemu_available(binary: &str) -> Result<()> {
    let output = Command::new("which")
        .arg(binary)
        .output()
        .map_err(|e| anyhow!("Failed to check QEMU availability: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("QEMU binary '{}' not found in PATH", binary));
    }
    
    Ok(())
}

/// Get QEMU version
pub fn get_qemu_version(binary: &str) -> Result<String> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| anyhow!("Failed to get QEMU version: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to execute '{} --version'", binary));
    }
    
    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dry_run() {
        let executor = QemuExecutor::new(
            "qemu-system-x86_64".to_string(),
            vec!["-m".to_string(), "1024M".to_string()].into()
        );
        
        let cmd = executor.dry_run();
        assert_eq!(cmd, "qemu-system-x86_64 -m 1024M");
    }
    
    #[test]
    fn test_check_qemu_available() {
        // This will fail if qemu-system-x86_64 is not installed
        let result = check_qemu_available("qemu-system-x86_64");
        // We don't assert success since it depends on the system
        let _ = result;
    }
}