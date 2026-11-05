//! IPC client for the elevated `flow8-ble-helper` (Windows DirectHCI).

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::sync::mpsc;

pub const PIPE_NAME: &str = r"\\.\pipe\Flow8MidiBle\v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Connect,
    Cancel,
    Write { hex: String },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Ready,
    State { state: String },
    Notify { hex: String },
    Error { message: String },
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return Err("hex length must be even".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn encode_line(msg: &ClientMsg) -> Result<String, String> {
    let mut line = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    line.push('\n');
    Ok(line)
}

pub fn helper_exe_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("flow8-ble-helper.exe");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("flow8-ble-helper.exe")
}

/// Launch the helper elevated (UAC). Returns immediately after ShellExecute.
pub fn launch_elevated() -> Result<(), String> {
    let path = helper_exe_path();
    if !path.is_file() {
        return Err(format!(
            "flow8-ble-helper.exe not found at {} — reinstall FLOW 8 MIDI Controller",
            path.display()
        ));
    }

    use std::os::windows::ffi::OsStrExt;
    use windows::core::{PCWSTR, w};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: NUL-terminated path; SW_HIDE; runas triggers UAC consent UI.
    let result = unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("runas"),
            PCWSTR(path_wide.as_ptr()),
            w!("--trust-certificate"),
            PCWSTR::null(),
            SW_HIDE,
        )
    };
    if result.0 as usize <= 32 {
        return Err(format!(
            "failed to launch helper elevated (ShellExecute={})",
            result.0 as usize
        ));
    }
    Ok(())
}

pub struct HelperPipe {
    writer: tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>,
    reader: BufReader<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>>,
}

impl HelperPipe {
    pub async fn connect_with_retry(timeout: Duration) -> Result<Self, String> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            match ClientOptions::new().open(PIPE_NAME) {
                Ok(client) => {
                    let (reader, writer) = tokio::io::split(client);
                    return Ok(Self {
                        writer,
                        reader: BufReader::new(reader),
                    });
                }
                Err(e) => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err(format!(
                            "timed out waiting for helper pipe ({PIPE_NAME}): {e}"
                        ));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }

    pub async fn send(&mut self, msg: &ClientMsg) -> Result<(), String> {
        let line = encode_line(msg)?;
        self.writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("pipe write: {e}"))?;
        self.writer
            .flush()
            .await
            .map_err(|e| format!("pipe flush: {e}"))?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Option<ServerMsg>, String> {
        let mut line = String::new();
        let n = self
            .reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("pipe read: {e}"))?;
        if n == 0 {
            return Ok(None);
        }
        let msg = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
        Ok(Some(msg))
    }
}

/// Forward write requests onto the helper pipe until the channel closes.
pub async fn write_forward_loop(
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    pipe_tx: mpsc::Sender<ClientMsg>,
) {
    while let Some(bytes) = write_rx.recv().await {
        let msg = ClientMsg::Write {
            hex: hex_encode(&bytes),
        };
        if pipe_tx.send(msg).await.is_err() {
            break;
        }
    }
}
