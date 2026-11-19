use super::{ClipboardStamp, CopySource};
use std::mem::size_of;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use windows_sys::Win32::{
    Foundation::HWND,
    System::DataExchange::{GetClipboardOwner, GetClipboardSequenceNumber},
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LCONTROL, VK_LWIN, VK_MENU, VK_RWIN,
            VK_SHIFT,
        },
        WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        },
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::desktop) struct Foreground {
    // HWND 只用作标识，不持有或解引用该窗口。每次系统操作前重新核对。
    window: usize,
    process: u32,
    focus: usize,
    focus_process: Option<u32>,
}

impl Foreground {
    pub(in crate::desktop) fn current() -> Option<Self> {
        let window = unsafe { GetForegroundWindow() };
        let process = process_id(window)?;
        let mut gui = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..GUITHREADINFO::default()
        };
        let focus = if unsafe { GetGUIThreadInfo(0, &mut gui) } != 0 {
            gui.hwndFocus
        } else {
            std::ptr::null_mut()
        };
        let target = Self {
            window: window as usize,
            process,
            focus: focus as usize,
            focus_process: process_id(focus),
        };
        (unsafe { GetForegroundWindow() } == window).then_some(target)
    }

    pub(in crate::desktop) fn is_current(self) -> bool {
        Self::current() == Some(self)
    }
}

fn process_id(window: HWND) -> Option<u32> {
    if window.is_null() {
        return None;
    }
    let mut process = 0;
    let thread = unsafe { GetWindowThreadProcessId(window, &mut process) };
    (thread != 0 && process != 0).then_some(process)
}

pub(in crate::desktop) struct WindowsCopy<'a> {
    pub target: Foreground,
    pub app: &'a AppHandle,
}

impl CopySource for WindowsCopy<'_> {
    fn target_is_current(&self) -> bool {
        self.target.is_current()
    }

    fn copy_keys_released(&self) -> bool {
        [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_C]
            .into_iter()
            .all(|key| (unsafe { GetAsyncKeyState(i32::from(key)) }) >= 0)
    }

    fn clipboard_sequence(&self) -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    fn target_clipboard_stamp(&self) -> Option<ClipboardStamp> {
        let sequence = self.clipboard_sequence();
        let owner = unsafe { GetClipboardOwner() };
        let process = process_id(owner)?;
        // 剪贴板拥有者可能是应用的隐藏窗口或负责焦点控件的进程。
        if sequence == 0
            || (process != self.target.process && Some(process) != self.target.focus_process)
            || self.clipboard_sequence() != sequence
        {
            return None;
        }
        Some(ClipboardStamp {
            sequence,
            owner: owner as usize,
        })
    }

    fn send_copy(&self) -> bool {
        let events = [
            key(VK_LCONTROL, false),
            key(VK_C, false),
            key(VK_C, true),
            key(VK_LCONTROL, true),
        ];
        let sent = send(&events);
        if sent < events.len() {
            // 系统只接收部分事件时，尽量释放已经合成的按键。
            match sent {
                1 | 3 => {
                    send(&events[3..]);
                }
                2 => {
                    send(&events[2..]);
                }
                _ => {}
            }
            return false;
        }
        true
    }

    fn read_text(&self) -> Option<String> {
        self.app.clipboard().read_text().ok()
    }
}

fn key(code: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                ..KEYBDINPUT::default()
            },
        },
    }
}

fn send(events: &[INPUT]) -> usize {
    // Win32 在调用期间读取完整 INPUT 数组；长度与元素大小对应其 ABI。
    unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            size_of::<INPUT>() as i32,
        ) as usize
    }
}
