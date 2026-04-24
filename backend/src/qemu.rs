use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::process::{Child, Command};
use uuid::Uuid;

pub enum NetworkConfig {
    User { ssh_port: u16 },
    Bridge { mac_address: String },
}

pub fn mac_from_uuid(id: &str) -> String {
    let uuid = Uuid::parse_str(id).expect("valid uuid");
    let b = uuid.as_bytes();
    format!("52:54:00:{:02x}:{:02x}:{:02x}", b[0], b[1], b[2])
}

/// Returns true if a process with the given PID is currently alive.
/// Uses signal 0, which checks existence without sending any signal.
pub fn is_process_running(pid: u32) -> bool {
    // PIDs larger than i32::MAX would wrap to negative values, which have
    // special meaning to kill() (e.g. -1 means "all user processes").
    // No real process can have such a large PID, so treat as not running.
    if pid > i32::MAX as u32 {
        return false;
    }
    matches!(
        kill(Pid::from_raw(pid as i32), None),
        Ok(()) | Err(Errno::EPERM)
    )
}

/// Send a single command to a running QEMU human-monitor socket.
/// Drains the QEMU greeting banner before writing to avoid stalling the
/// socket buffer.
pub async fn send_monitor_command(socket_path: &str, command: &str) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let mut stream = UnixStream::connect(socket_path).await?;

    // Drain the QEMU greeting ("QEMU … monitor …\r\n(qemu) ") with a short
    // timeout so we don't block if the VM is slow to respond.
    let mut banner = vec![0u8; 512];
    let _ = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        stream.read(&mut banner),
    )
    .await;

    stream.write_all(format!("{command}\n").as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

pub fn vm_start(
    qcow2_file: &str,
    network: &NetworkConfig,
    monitor_socket: &str,
) -> Result<Child, std::io::Error> {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-m",
        "8192",
        "-smp",
        "6",
        "-drive",
        &format!("file={qcow2_file}"),
        "-boot",
        "d",
        "-vga",
        "virtio",
        "-nographic",
        "-monitor",
        &format!("unix:{monitor_socket},server,nowait"),
    ]);

    match network {
        NetworkConfig::User { ssh_port } => {
            cmd.args([
                "-netdev",
                &format!("user,id=net0,hostfwd=tcp::{ssh_port}-:22"),
                "-device",
                "e1000,netdev=net0",
            ]);
        }
        NetworkConfig::Bridge { mac_address } => {
            cmd.args([
                "-netdev",
                "bridge,id=net0,br=br0",
                "-device",
                &format!("e1000,netdev=net0,mac={mac_address}"),
            ]);
        }
    }

    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_is_process_running_current_process() {
        assert!(is_process_running(std::process::id()));
    }

    #[test]
    fn test_is_process_running_invalid_pid() {
        // PID u32::MAX is effectively guaranteed not to exist.
        assert!(!is_process_running(u32::MAX));
    }

    #[tokio::test]
    async fn test_send_monitor_command_sends_command_to_socket() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixListener;

        let tmp = tempfile::TempDir::new().unwrap();
        let socket_path = tmp.path().join("test.monitor");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Simulate QEMU greeting
            stream
                .write_all(b"QEMU 7.2.0 monitor - type 'help'\r\n(qemu) ")
                .await
                .unwrap();
            // Read the command sent by the client
            let mut buf = vec![0u8; 64];
            if let Ok(n) = stream.read(&mut buf).await {
                *received_clone.lock().await = buf[..n].to_vec();
            }
        });

        tokio::task::yield_now().await;

        send_monitor_command(socket_path.to_str().unwrap(), "system_powerdown")
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let data = received.lock().await;
        assert_eq!(String::from_utf8_lossy(&data), "system_powerdown\n");
    }

    #[tokio::test]
    async fn test_send_monitor_command_returns_error_for_missing_socket() {
        let result = send_monitor_command("/nonexistent/path.monitor", "stop").await;
        assert!(result.is_err());
    }
}
