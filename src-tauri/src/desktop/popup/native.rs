use super::WINDOW_ERROR;
use tauri::{WebviewWindow, Window};

pub(super) fn foreground(window: &Window) -> bool {
    #[cfg(windows)]
    {
        // WebView 内部控件切换焦点时，顶层前台窗口仍是同一个 HWND。
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        window
            .hwnd()
            .is_ok_and(|hwnd| unsafe { GetForegroundWindow() } == hwnd.0)
    }
    #[cfg(not(windows))]
    window.is_focused().unwrap_or(false)
}

pub(super) fn pin(window: &WebviewWindow, pinned: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        let hwnd = window.hwnd().map_err(|_| WINDOW_ERROR)?;
        let order = if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
        // 取实际置顶操作的返回值，保持原应用焦点，并在失败时保留会话中的图钉状态。
        if unsafe {
            SetWindowPos(
                hwnd.0,
                order,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            )
        } == 0
        {
            return Err(WINDOW_ERROR.into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    window
        .set_always_on_top(pinned)
        .map_err(|_| WINDOW_ERROR.into())
}
