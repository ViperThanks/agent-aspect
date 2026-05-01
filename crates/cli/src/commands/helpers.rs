//! CLI 子命令共享辅助函数。
//!
//! 提供二进制目录定位和 launchctl 调用封装，
//! 被 launchd、bridge、daemon、init 等多个命令模块复用。

/// 返回当前可执行文件所在目录。
///
/// 约定：checkpoint / checkpointd / checkpoint-hook / checkpoint-bridge
/// 都安装在同一个目录下，所以通过当前二进制路径定位兄弟可执行文件。
pub fn bin_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
}

/// 执行 `launchctl <subcmd> <args..>`，成功返回 stdout，失败返回 stderr。
///
/// macOS launchctl 的 stdout/stderr 语义不太一致：
/// 有些错误信息输出到 stdout，所以失败时优先取 stderr，
/// stderr 为空时才取 stdout。
pub fn run_launchctl(subcmd: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("launchctl");
    cmd.arg(subcmd);
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().map_err(|e| format!("exec launchctl: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}
