use anyhow::{anyhow, Result};
use std::mem::size_of;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, VIRTUAL_KEY,
};
use super::OutputAction;

pub struct KeyPressAction {
    pub key: u16,            // Virtual key code
    pub modifiers: Vec<u16>, // Modifier virtual key codes
}

pub fn parse_virtual_key(name: &str) -> Result<u16> {
    let upper = name.to_uppercase();
    match upper.as_str() {
        "F1" => Ok(0x70),
        "F2" => Ok(0x71),
        "F3" => Ok(0x72),
        "F4" => Ok(0x73),
        "F5" => Ok(0x74),
        "F6" => Ok(0x75),
        "F7" => Ok(0x76),
        "F8" => Ok(0x77),
        "F9" => Ok(0x78),
        "F10" => Ok(0x79),
        "F11" => Ok(0x7A),
        "F12" => Ok(0x7B),
        "F13" => Ok(0x7C),
        "F14" => Ok(0x7D),
        "F15" => Ok(0x7E),
        "F16" => Ok(0x7F),
        "F17" => Ok(0x80),
        "F18" => Ok(0x81),
        "F19" => Ok(0x82),
        "F20" => Ok(0x83),
        "F21" => Ok(0x84),
        "F22" => Ok(0x85),
        "F23" => Ok(0x86),
        "F24" => Ok(0x87),
        "SPACE" => Ok(0x20),
        "ENTER" | "RETURN" => Ok(0x0D),
        "TAB" => Ok(0x09),
        "ESCAPE" | "ESC" => Ok(0x1B),
        "CTRL" | "CONTROL" => Ok(0x11), // VK_CONTROL
        "SHIFT" => Ok(0x10), // VK_SHIFT
        "ALT" => Ok(0x12), // VK_MENU
        "LEFT" => Ok(0x25),
        "UP" => Ok(0x26),
        "RIGHT" => Ok(0x27),
        "DOWN" => Ok(0x28),
        "PAGEUP" | "PAGE UP" => Ok(0x21),
        "PAGEDOWN" | "PAGE DOWN" => Ok(0x22),
        "END" => Ok(0x23),
        "HOME" => Ok(0x24),
        "INSERT" => Ok(0x2D),
        "DELETE" => Ok(0x2E),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                Ok(c as u16)
            } else {
                Err(anyhow!("Unsupported key character: {}", s))
            }
        }
        _ => Err(anyhow!("Unknown virtual key: {}", name)),
    }
}

fn is_extended_key(vk: u16) -> bool {
    matches!(
        vk,
        0x21 | // VK_PRIOR (Page Up)
        0x22 | // VK_NEXT (Page Down)
        0x23 | // VK_END
        0x24 | // VK_HOME
        0x25 | // VK_LEFT
        0x26 | // VK_UP
        0x27 | // VK_RIGHT
        0x28 | // VK_DOWN
        0x2D | // VK_INSERT
        0x2E   // VK_DELETE
    )
}

#[cfg(windows)]
fn make_key_input(vk: u16, keyup: bool) -> INPUT {
    let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
    if keyup {
        flags |= KEYEVENTF_KEYUP;
    }
    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

impl OutputAction for KeyPressAction {
    fn execute(&self) -> Result<()> {
        #[cfg(windows)]
        {
            let mut inputs: Vec<INPUT> = Vec::new();

            // Press modifiers
            for &modifier in &self.modifiers {
                inputs.push(make_key_input(modifier, false)); // keydown
            }

            // Press and release main key
            inputs.push(make_key_input(self.key, false)); // keydown
            inputs.push(make_key_input(self.key, true));  // keyup

            // Release modifiers (reverse order)
            for &modifier in self.modifiers.iter().rev() {
                inputs.push(make_key_input(modifier, true)); // keyup
            }

            // Send all at once
            let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
            if sent != inputs.len() as u32 {
                return Err(anyhow!("SendInput only sent {}/{} events", sent, inputs.len()));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "key_press"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_virtual_key() {
        assert_eq!(parse_virtual_key("a").unwrap(), 0x41);
        assert_eq!(parse_virtual_key("A").unwrap(), 0x41);
        assert_eq!(parse_virtual_key("z").unwrap(), 0x5A);
        assert_eq!(parse_virtual_key("0").unwrap(), 0x30);
        assert_eq!(parse_virtual_key("9").unwrap(), 0x39);
        assert_eq!(parse_virtual_key("f1").unwrap(), 0x70);
        assert_eq!(parse_virtual_key("Ctrl").unwrap(), 0x11);
        assert_eq!(parse_virtual_key("ESCAPE").unwrap(), 0x1B);
        assert_eq!(parse_virtual_key("Right").unwrap(), 0x27);
        assert!(parse_virtual_key("unknown").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_make_key_input() {
        let input = make_key_input(0x41, false);
        assert_eq!(input.r#type, INPUT_KEYBOARD);
        unsafe {
            assert_eq!(input.Anonymous.ki.wVk.0, 0x41);
            assert_eq!(input.Anonymous.ki.dwFlags.0, 0); // No flags
        }

        let input_up = make_key_input(0x41, true);
        unsafe {
            assert_eq!(input_up.Anonymous.ki.dwFlags.0, KEYEVENTF_KEYUP.0);
        }

        let input_ext = make_key_input(0x27, false); // Right arrow
        unsafe {
            assert_eq!(input_ext.Anonymous.ki.dwFlags.0, KEYEVENTF_EXTENDEDKEY.0);
        }
    }
}
