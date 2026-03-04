use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const LOG_FILENAME: &str = "flow8-midi.log";
const RING_BUFFER_CAPACITY: usize = 2000;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] [{}] {}", self.timestamp, self.level, self.message)
    }
}

fn timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

fn system_diagnostics() -> String {
    format!(
        "OS: {} {} ({})\nHostname: {}\nApp: FLOW 8 MIDI Controller v{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY,
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        env!("CARGO_PKG_VERSION"),
    )
}

pub fn init() {
    LOG_BUFFER
        .set(Mutex::new(VecDeque::with_capacity(RING_BUFFER_CAPACITY)))
        .ok();

    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(LOG_FILENAME)))
        .unwrap_or_else(|| PathBuf::from(LOG_FILENAME));

    let header = format!(
        "=== FLOW 8 MIDI Controller v{} ===\n{}",
        env!("CARGO_PKG_VERSION"),
        system_diagnostics()
    );

    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
    {
        let _ = writeln!(file, "[{}] {}", timestamp(), header);
    }

    LOG_PATH.set(path).ok();

    log_with_level(LogLevel::Info, &header);
}

fn push_to_buffer(entry: LogEntry) {
    if let Some(buffer) = LOG_BUFFER.get() {
        if let Ok(mut buf) = buffer.lock() {
            if buf.len() >= RING_BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }
}

pub fn log_with_level(level: LogLevel, message: &str) {
    let ts = timestamp();
    let entry = LogEntry {
        timestamp: ts.clone(),
        level: level.clone(),
        message: message.to_string(),
    };

    eprintln!("{}", entry);
    push_to_buffer(entry);

    if let Some(path) = LOG_PATH.get() {
        if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(path) {
            let _ = writeln!(file, "[{}] [{}] {}", ts, level, message);
        }
    }
}

pub fn log(message: &str) {
    log_with_level(LogLevel::Info, message);
}

pub fn export_log() -> String {
    let mut output = String::new();
    output.push_str(&format!("--- Debug Report ---\n{}\n\n", system_diagnostics()));

    if let Some(buffer) = LOG_BUFFER.get() {
        if let Ok(buf) = buffer.lock() {
            for entry in buf.iter() {
                output.push_str(&format!("{}\n", entry));
            }
        }
    }

    output.push_str("--- End of Report ---\n");
    output
}

pub fn get_recent_entries(count: usize) -> Vec<LogEntry> {
    if let Some(buffer) = LOG_BUFFER.get() {
        if let Ok(buf) = buffer.lock() {
            return buf.iter().rev().take(count).cloned().collect::<Vec<_>>().into_iter().rev().collect();
        }
    }
    vec![]
}

pub fn entry_count() -> usize {
    if let Some(buffer) = LOG_BUFFER.get() {
        if let Ok(buf) = buffer.lock() {
            return buf.len();
        }
    }
    0
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logger::log(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::log_with_level($crate::logger::LogLevel::Debug, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::log_with_level($crate::logger::LogLevel::Warn, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::log_with_level($crate::logger::LogLevel::Error, &format!($($arg)*))
    };
}
