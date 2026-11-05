use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
const LOCK_FILE: &str = "lock";
#[cfg(not(target_os = "windows"))]
const APP_DIR: &str = ".flow8-midi";

#[cfg(target_os = "windows")]
pub fn ensure_single_instance() -> Result<SingleInstanceGuard, String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            attrs: *const std::ffi::c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }

    const ERROR_ALREADY_EXISTS: u32 = 183;

    let name: Vec<u16> = OsStr::new("Global\\Flow8MidiController")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err("Failed to create mutex".to_string());
    }

    let err = unsafe { GetLastError() };
    if err == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return Err("Another instance is already running".to_string());
    }

    Ok(SingleInstanceGuard {
        _handle: Some(handle as usize),
        _lock_path: None,
    })
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_single_instance() -> Result<SingleInstanceGuard, String> {
    use std::fs;
    use std::io::Write;

    let dir = dirs::home_dir()
        .ok_or("Cannot resolve home directory")?
        .join(APP_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create dir: {}", e))?;

    let lock_path = dir.join(LOCK_FILE);

    if lock_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&lock_path) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if process_alive(pid) {
                    return Err("Another instance is already running".to_string());
                }
            }
        }
    }

    let mut file =
        fs::File::create(&lock_path).map_err(|e| format!("Cannot create lock: {}", e))?;
    write!(file, "{}", std::process::id()).map_err(|e| format!("Cannot write lock: {}", e))?;

    Ok(SingleInstanceGuard {
        _handle: None,
        _lock_path: Some(lock_path),
    })
}

#[cfg(not(target_os = "windows"))]
fn process_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
        || libc_kill(pid as i32, 0) == 0
}

#[cfg(not(target_os = "windows"))]
fn libc_kill(pid: i32, sig: i32) -> i32 {
    #[cfg(target_family = "unix")]
    {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        unsafe { kill(pid, sig) }
    }
    #[cfg(not(target_family = "unix"))]
    {
        let _ = (pid, sig);
        -1
    }
}

pub struct SingleInstanceGuard {
    _handle: Option<usize>,
    _lock_path: Option<PathBuf>,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(handle) = self._handle {
            #[link(name = "kernel32")]
            extern "system" {
                fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                CloseHandle(handle as *mut std::ffi::c_void);
            }
        }

        #[cfg(not(target_os = "windows"))]
        if let Some(ref path) = self._lock_path {
            let _ = std::fs::remove_file(path);
        }
    }
}
