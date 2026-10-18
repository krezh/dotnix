use std::{
    fs::File,
    io::{self, Write},
    os::fd::{AsFd, FromRawFd},
    time::Instant,
};

use wayland_client::{QueueHandle, protocol::wl_seat::WlSeat};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// The keymap the virtual keyboard types with; the compositor fills in the includes.
const KEYMAP: &str = "xkb_keymap {
    xkb_keycodes { include \"evdev+aliases(qwerty)\" };
    xkb_types { include \"complete\" };
    xkb_compat { include \"complete\" };
    xkb_symbols { include \"pc+us+inet(evdev)\" };
};
";
/// `wl_keyboard.keymap_format.xkb_v1`.
const XKB_V1: u32 = 1;
const PRESSED: u32 = 1;
const RELEASED: u32 = 0;

/// Modifiers by name: their evdev key code and their bit in the keymap's modifier mask.
const MODIFIERS: [(&str, u32, u32); 6] = [
    ("ctrl", 29, 1 << 2),
    ("control", 29, 1 << 2),
    ("shift", 42, 1),
    ("alt", 56, 1 << 3),
    ("super", 125, 1 << 6),
    ("logo", 125, 1 << 6),
];
/// The letters' evdev key codes, row by row.
const LETTER_ROWS: [(&str, u32); 3] = [("qwertyuiop", 16), ("asdfghjkl", 30), ("zxcvbnm", 44)];
const INSERT: u32 = 110;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    /// Key code and mask bit, in the order they are pressed.
    modifiers: Vec<(u32, u32)>,
    key: u32,
}

impl Shortcut {
    /// `None` for an empty one.
    pub fn parse(text: &str) -> Result<Option<Self>, String> {
        let text = text.trim().to_lowercase();
        if text.is_empty() {
            return Ok(None);
        }
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts.pop().unwrap_or_default();
        let modifiers = parts
            .iter()
            .map(|name| {
                MODIFIERS
                    .iter()
                    .find(|(modifier, _, _)| modifier == name)
                    .map(|&(_, code, bit)| (code, bit))
                    .ok_or_else(|| format!("unknown modifier '{name}' in '{text}'"))
            })
            .collect::<Result<_, _>>()?;
        let key = match key {
            "insert" => INSERT,
            letter if letter.len() == 1 => LETTER_ROWS
                .iter()
                .find_map(|(row, first)| row.find(letter).map(|index| first + index as u32))
                .ok_or_else(|| format!("unknown key '{letter}' in '{text}'"))?,
            other => {
                return Err(format!(
                    "unknown key '{other}' in '{text}'; use a letter or insert"
                ));
            }
        };
        Ok(Some(Self { modifiers, key }))
    }
}

pub struct Keyboard {
    keyboard: ZwpVirtualKeyboardV1,
    started: Instant,
}

impl Keyboard {
    pub fn new<D>(
        manager: &ZwpVirtualKeyboardManagerV1,
        seat: &WlSeat,
        queue: &QueueHandle<D>,
    ) -> io::Result<Self>
    where
        D: wayland_client::Dispatch<ZwpVirtualKeyboardV1, ()> + 'static,
    {
        let keyboard = manager.create_virtual_keyboard(seat, queue, ());
        let fd = unsafe { libc::memfd_create(c"island-keymap".as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut keymap = unsafe { File::from_raw_fd(fd) };
        keymap.write_all(KEYMAP.as_bytes())?;
        keymap.write_all(&[0])?;
        keyboard.keymap(XKB_V1, keymap.as_fd(), KEYMAP.len() as u32 + 1);
        Ok(Self {
            keyboard,
            started: Instant::now(),
        })
    }

    pub fn press(&self, shortcut: &Shortcut) {
        let time = u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let mut mask = 0;
        for &(code, bit) in &shortcut.modifiers {
            self.keyboard.key(time, code, PRESSED);
            mask |= bit;
            self.keyboard.modifiers(mask, 0, 0, 0);
        }
        self.keyboard.key(time, shortcut.key, PRESSED);
        self.keyboard.key(time, shortcut.key, RELEASED);
        for &(code, bit) in shortcut.modifiers.iter().rev() {
            self.keyboard.key(time, code, RELEASED);
            mask &= !bit;
            self.keyboard.modifiers(mask, 0, 0, 0);
        }
    }
}

#[cfg(test)]
#[path = "paste_test.rs"]
mod tests;
