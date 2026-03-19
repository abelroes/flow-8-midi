#[cfg(target_os = "windows")]
mod imp {
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;
    use std::sync::mpsc;
    use std::thread;

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CREATESTRUCTW, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
        DestroyMenu, DispatchMessageW, FindWindowW, GetClassLongPtrW, GetCursorPos, GetMessageW,
        IDC_ARROW, LoadCursorW, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
        SendMessageW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TPM_RIGHTBUTTON,
        TrackPopupMenu, TranslateMessage, WM_APP, WM_COMMAND, WM_CREATE, WM_DESTROY,
        WM_GETICON, WM_LBUTTONUP, WM_NCCREATE, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
        GWLP_USERDATA, GCLP_HICON, GCLP_HICONSM, ICON_BIG, ICON_SMALL, ICON_SMALL2, SW_HIDE,
        SW_RESTORE,
    };

    const APP_WINDOW_TITLE: &str = "FLOW 8 MIDI Controller";
    const TRAY_WINDOW_CLASS: &str = "Flow8MidiTrayWindow";
    const TRAY_ICON_ID: u32 = 1;
    const MENU_RESTORE_ID: usize = 1001;
    const MENU_EXIT_ID: usize = 1002;
    const TRAY_CALLBACK_MSG: u32 = WM_APP + 1;
    const TRAY_CONTROL_MSG: u32 = WM_APP + 2;

    #[derive(Debug, Clone, Copy)]
    pub enum TrayEvent {
        RestoreRequested,
        ExitRequested,
    }

    #[derive(Debug, Clone, Copy)]
    enum TrayCommand {
        ShowIcon = 1,
        HideIcon = 2,
        Shutdown = 3,
    }

    struct TrayWindowState {
        event_tx: mpsc::Sender<TrayEvent>,
        menu: usize,
        icon_visible: bool,
    }

    pub struct TrayManager {
        hwnd: usize,
        event_rx: mpsc::Receiver<TrayEvent>,
    }

    impl TrayManager {
        pub fn new() -> Result<Self, String> {
            let (hwnd_tx, hwnd_rx) = mpsc::channel();
            let (event_tx, event_rx) = mpsc::channel();

            thread::Builder::new()
                .name("flow8-tray".to_string())
                .spawn(move || tray_thread(event_tx, hwnd_tx))
                .map_err(|e| format!("Failed to spawn tray thread: {}", e))?;

            let hwnd = hwnd_rx
                .recv()
                .map_err(|e| format!("Tray initialization failed: {}", e))?;

            Ok(Self { hwnd, event_rx })
        }

        pub fn show_icon(&self) {
            unsafe {
                PostMessageW(self.hwnd as HWND, TRAY_CONTROL_MSG, TrayCommand::ShowIcon as WPARAM, 0);
            }
        }

        pub fn hide_icon(&self) {
            unsafe {
                PostMessageW(self.hwnd as HWND, TRAY_CONTROL_MSG, TrayCommand::HideIcon as WPARAM, 0);
            }
        }

        pub fn shutdown(&self) {
            unsafe {
                PostMessageW(self.hwnd as HWND, TRAY_CONTROL_MSG, TrayCommand::Shutdown as WPARAM, 0);
            }
        }

        pub fn try_recv(&self) -> Option<TrayEvent> {
            self.event_rx.try_recv().ok()
        }
    }

    pub fn hide_app_window() {
        unsafe {
            let hwnd = app_window();
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    pub fn show_app_window() {
        unsafe {
            let hwnd = app_window();
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
    }

    fn tray_thread(event_tx: mpsc::Sender<TrayEvent>, hwnd_tx: mpsc::Sender<usize>) {
        unsafe {
            let instance = GetModuleHandleW(null_mut());
            if instance.is_null() {
                let _ = hwnd_tx.send(0);
                return;
            }

            let class_name = wide_null(TRAY_WINDOW_CLASS);
            let wc = WNDCLASSW {
                lpfnWndProc: Some(tray_wnd_proc),
                hInstance: instance,
                lpszClassName: class_name.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                ..zeroed()
            };

            RegisterClassW(&wc);

            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, MENU_RESTORE_ID, wide_null("Open FLOW 8 MIDI Controller").as_ptr());
            AppendMenuW(menu, MF_STRING, MENU_EXIT_ID, wide_null("Exit").as_ptr());

            let state = Box::new(TrayWindowState {
                event_tx,
                menu: menu as usize,
                icon_visible: false,
            });
            let state_ptr = Box::into_raw(state);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide_null("FLOW 8 Tray").as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                state_ptr.cast(),
            );

            let _ = hwnd_tx.send(hwnd as usize);

            if hwnd.is_null() {
                let _ = Box::from_raw(state_ptr);
                return;
            }

            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    unsafe extern "system" fn tray_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCCREATE => {
                let create = &*(lparam as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                1
            }
            WM_CREATE => 0,
            TRAY_CALLBACK_MSG => {
                let Some(state) = state_mut(hwnd) else {
                    return 0;
                };

                match lparam as u32 {
                    WM_LBUTTONUP => {
                        let _ = state.event_tx.send(TrayEvent::RestoreRequested);
                    }
                    WM_RBUTTONUP => {
                        let mut point = POINT { x: 0, y: 0 };
                        GetCursorPos(&mut point);
                        SetForegroundWindow(hwnd);
                        TrackPopupMenu(state.menu as _, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, null_mut());
                    }
                    _ => {}
                }
                0
            }
            WM_COMMAND => {
                let Some(state) = state_mut(hwnd) else {
                    return 0;
                };

                match (wparam & 0xFFFF) as usize {
                    MENU_RESTORE_ID => {
                        let _ = state.event_tx.send(TrayEvent::RestoreRequested);
                    }
                    MENU_EXIT_ID => {
                        let _ = state.event_tx.send(TrayEvent::ExitRequested);
                    }
                    _ => {}
                }
                0
            }
            TRAY_CONTROL_MSG => {
                if let Some(state) = state_mut(hwnd) {
                    match wparam as u32 {
                        x if x == TrayCommand::ShowIcon as u32 => add_tray_icon(hwnd, state),
                        x if x == TrayCommand::HideIcon as u32 => remove_tray_icon(hwnd, state),
                        x if x == TrayCommand::Shutdown as u32 => {
                            remove_tray_icon(hwnd, state);
                            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                        }
                        _ => {}
                    }
                }
                0
            }
            WM_DESTROY => {
                if let Some(state) = state_mut(hwnd) {
                    remove_tray_icon(hwnd, state);
                    if state.menu != 0 {
                        DestroyMenu(state.menu as _);
                    }
                }

                let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                ) as *mut TrayWindowState;

                if !ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    let _ = Box::from_raw(ptr);
                }

                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn state_mut(hwnd: HWND) -> Option<&'static mut TrayWindowState> {
        let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
        ) as *mut TrayWindowState;
        ptr.as_mut()
    }

    unsafe fn add_tray_icon(hwnd: HWND, state: &mut TrayWindowState) {
        if state.icon_visible {
            return;
        }

        let mut data = tray_data(hwnd);
        if Shell_NotifyIconW(NIM_ADD, &mut data) != 0 {
            state.icon_visible = true;
        }
    }

    unsafe fn remove_tray_icon(hwnd: HWND, state: &mut TrayWindowState) {
        if !state.icon_visible {
            return;
        }

        let mut data = tray_data(hwnd);
        Shell_NotifyIconW(NIM_DELETE, &mut data);
        state.icon_visible = false;
    }

    unsafe fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = zeroed();
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_ID;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_CALLBACK_MSG;
        data.hIcon = app_icon_handle();

        let tooltip = wide_null(APP_WINDOW_TITLE);
        for (idx, ch) in tooltip.iter().take(data.szTip.len()).enumerate() {
            data.szTip[idx] = *ch;
        }

        data
    }

    unsafe fn app_window() -> HWND {
        FindWindowW(null_mut(), wide_null(APP_WINDOW_TITLE).as_ptr())
    }

    unsafe fn app_icon_handle() -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
        let hwnd = app_window();
        if hwnd.is_null() {
            return null_mut();
        }

        let mut icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON =
            SendMessageW(hwnd, WM_GETICON, ICON_SMALL2 as usize, 0) as _;
        if icon.is_null() {
            icon = SendMessageW(hwnd, WM_GETICON, ICON_SMALL as usize, 0) as _;
        }
        if icon.is_null() {
            icon = SendMessageW(hwnd, WM_GETICON, ICON_BIG as usize, 0) as _;
        }
        if icon.is_null() {
            icon = GetClassLongPtrW(hwnd, GCLP_HICONSM) as _;
        }
        if icon.is_null() {
            icon = GetClassLongPtrW(hwnd, GCLP_HICON) as _;
        }

        icon
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    #[derive(Debug, Clone, Copy)]
    pub enum TrayEvent {
        RestoreRequested,
        ExitRequested,
    }

    pub struct TrayManager;

    impl TrayManager {
        pub fn new() -> Result<Self, String> {
            Err("System tray is only supported on Windows in this implementation".to_string())
        }

        pub fn show_icon(&self) {}
        pub fn hide_icon(&self) {}
        pub fn shutdown(&self) {}
        pub fn try_recv(&self) -> Option<TrayEvent> {
            None
        }
    }

    pub fn hide_app_window() {}
    pub fn show_app_window() {}
}

pub use imp::*;
