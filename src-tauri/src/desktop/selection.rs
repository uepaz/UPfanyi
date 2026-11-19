#[cfg(windows)]
pub(super) mod windows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ClipboardStamp {
    pub sequence: u32,
    pub owner: usize,
}

pub(super) trait CopySource {
    fn target_is_current(&self) -> bool;
    fn copy_keys_released(&self) -> bool;
    fn clipboard_sequence(&self) -> u32;
    fn target_clipboard_stamp(&self) -> Option<ClipboardStamp>;
    fn send_copy(&self) -> bool;
    fn read_text(&self) -> Option<String>;
}

pub(super) fn capture_text(
    source: &impl CopySource,
    active: impl Fn() -> bool,
    mut pause: impl FnMut(),
    attempts: usize,
) -> Option<String> {
    let valid = || active() && source.target_is_current();
    let mut ready = false;
    for _ in 0..attempts {
        if !valid() {
            return None;
        }
        if source.copy_keys_released() {
            ready = true;
            break;
        }
        pause();
    }
    if !ready {
        return None;
    }

    let before = source.clipboard_sequence();
    if !valid() || !source.copy_keys_released() || !source.send_copy() {
        return None;
    }
    for _ in 0..attempts {
        if !valid() {
            return None;
        }
        if let Some(stamp) = source
            .target_clipboard_stamp()
            .filter(|s| s.sequence != before)
        {
            let text = source.read_text().filter(|text| !text.trim().is_empty());
            // 读取和取得版本不是同一次系统调用。中途被改写时丢弃读到的内容。
            if !valid() {
                return None;
            }
            if text.is_some() && source.target_clipboard_stamp() == Some(stamp) {
                return text;
            }
        }
        pause();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
    };

    struct Fixture {
        current: Cell<bool>,
        released: Cell<bool>,
        copy_allowed: bool,
        stamps: RefCell<VecDeque<Option<ClipboardStamp>>>,
        text: Option<String>,
        actions: RefCell<Vec<&'static str>>,
    }

    impl Fixture {
        fn new(
            stamps: impl IntoIterator<Item = Option<ClipboardStamp>>,
            text: Option<&str>,
        ) -> Self {
            Self {
                current: Cell::new(true),
                released: Cell::new(true),
                copy_allowed: true,
                stamps: RefCell::new(stamps.into_iter().collect()),
                text: text.map(str::to_owned),
                actions: RefCell::new(Vec::new()),
            }
        }
    }

    impl CopySource for Fixture {
        fn target_is_current(&self) -> bool {
            self.current.get()
        }
        fn copy_keys_released(&self) -> bool {
            self.released.get()
        }
        fn clipboard_sequence(&self) -> u32 {
            self.actions.borrow_mut().push("baseline");
            10
        }
        fn target_clipboard_stamp(&self) -> Option<ClipboardStamp> {
            self.stamps.borrow_mut().pop_front().flatten()
        }
        fn send_copy(&self) -> bool {
            assert!(self.released.get());
            self.actions.borrow_mut().push("copy");
            self.copy_allowed
        }
        fn read_text(&self) -> Option<String> {
            self.actions.borrow_mut().push("read");
            self.text.clone()
        }
    }

    fn stamp(sequence: u32) -> Option<ClipboardStamp> {
        Some(ClipboardStamp {
            sequence,
            owner: 123,
        })
    }

    #[test]
    fn waits_for_release_and_reads_only_after_a_fresh_copy_even_if_text_matches_old_value() {
        let source = Fixture::new(
            [stamp(10), None, stamp(11), stamp(11)],
            Some("同一段文字\n"),
        );
        source.released.set(false);
        let result = capture_text(&source, || true, || source.released.set(true), 4);
        assert_eq!(result.as_deref(), Some("同一段文字\n"));
        assert_eq!(*source.actions.borrow(), ["baseline", "copy", "read"]);
    }

    #[test]
    fn old_contents_or_unrelated_clipboard_owners_are_never_read() {
        for stamps in [[stamp(10); 3], [None; 3]] {
            let source = Fixture::new(stamps, Some("旧剪贴板"));
            assert_eq!(capture_text(&source, || true, || {}, 3), None);
            assert_eq!(*source.actions.borrow(), ["baseline", "copy"]);
        }
    }

    #[test]
    fn empty_nontext_or_changed_during_read_does_not_translate() {
        for (stamps, text) in [
            ([stamp(11), stamp(11)], None),
            ([stamp(11), stamp(11)], Some("  \n")),
            ([stamp(11), stamp(12)], Some("被其他复制替换")),
        ] {
            let source = Fixture::new(stamps, text);
            assert_eq!(capture_text(&source, || true, || {}, 1), None);
            assert_eq!(*source.actions.borrow(), ["baseline", "copy", "read"]);
        }
    }

    #[test]
    fn copy_is_not_sent_when_focus_changes_or_keys_remain_held() {
        let source = Fixture::new([], Some("旧剪贴板"));
        source.released.set(false);
        assert_eq!(capture_text(&source, || true, || {}, 2), None);
        assert_eq!(
            capture_text(&source, || true, || source.current.set(false), 2),
            None
        );
        assert!(source.actions.borrow().is_empty());
    }

    #[test]
    fn rejected_input_and_cancelled_tasks_do_not_read_or_deliver_text() {
        let mut source = Fixture::new([stamp(11), stamp(11)], Some("新文字"));
        source.copy_allowed = false;
        assert_eq!(capture_text(&source, || true, || {}, 3), None);
        assert_eq!(*source.actions.borrow(), ["baseline", "copy"]);

        source.copy_allowed = true;
        source.actions.borrow_mut().clear();
        let active = || !source.actions.borrow().contains(&"read");
        assert_eq!(capture_text(&source, active, || {}, 3), None);
        assert_eq!(*source.actions.borrow(), ["baseline", "copy", "read"]);
    }
}
