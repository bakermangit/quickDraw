use anyhow::{anyhow, Result};
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::Sender;
#[cfg(windows)]
use windows::core::w;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    PostMessageW, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WM_INPUT,
    WM_USER, WNDCLASSW,
};

use crate::types::{InputEvent, InputEventType, MouseButton, VirtualKey};
use super::InputSource;

// HWND is not Send, but we only use it to send a quit message across threads.
#[cfg(windows)]
#[derive(Clone, Copy)]
struct SendHwnd(HWND);
#[cfg(windows)]
unsafe impl Send for SendHwnd {}
#[cfg(windows)]
unsafe impl Sync for SendHwnd {}

pub struct RawInputSource {
    thread_handle: Option<JoinHandle<()>>,
    #[cfg(windows)]
    window_handle: Option<SendHwnd>,
    #[cfg(not(windows))]
    window_handle: Option<()>,
    is_running: Arc<AtomicBool>,
    listen_mouse: bool,
    listen_keyboard: bool,
}

impl RawInputSource {
    pub fn new(listen_mouse: bool, listen_keyboard: bool) -> Self {
        Self {
            thread_handle: None,
            window_handle: None,
            is_running: Arc::new(AtomicBool::new(false)),
            listen_mouse,
            listen_keyboard,
        }
    }
}

impl InputSource for RawInputSource {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<()> {
        #[cfg(windows)]
        {
            if self.thread_handle.is_some() {
                return Err(anyhow!("RawInputSource is already running"));
            }

            self.is_running.store(true, Ordering::SeqCst);
            let is_running = Arc::clone(&self.is_running);

            let (hwnd_tx, hwnd_rx) = std::sync::mpsc::channel();

            let listen_mouse = self.listen_mouse;
            let listen_keyboard = self.listen_keyboard;

            let handle = thread::spawn(move || {
                if let Err(e) = run_message_loop(tx, hwnd_tx, is_running, listen_mouse, listen_keyboard) {
                    tracing::error!("Raw input message loop error: {}", e);
                }
            });

            self.thread_handle = Some(handle);

            if let Ok(hwnd) = hwnd_rx.recv() {
                self.window_handle = Some(hwnd);
            }

            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = tx;
            Err(anyhow!("RawInputSource is only supported on Windows"))
        }
    }

    fn stop(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            if let Some(SendHwnd(hwnd)) = self.window_handle.take() {
                self.is_running.store(false, Ordering::SeqCst);
                unsafe {
                    let _ = PostMessageW(hwnd, WM_USER + 1, WPARAM(0), LPARAM(0));
                }
            }
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    fn can_block(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "raw_input"
    }
}

#[cfg(windows)]
unsafe extern "system" fn raw_input_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(windows)]
fn run_message_loop(
    tx: Sender<InputEvent>,
    hwnd_tx: std::sync::mpsc::Sender<SendHwnd>,
    is_running: Arc<AtomicBool>,
    listen_mouse: bool,
    listen_keyboard: bool,
) -> Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class_name = w!("QuickDrawRawInputClass");

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(raw_input_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        let atom = RegisterClassW(&wnd_class);
        if atom == 0 {
            let err = windows::Win32::Foundation::GetLastError();
            if err != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS {
                return Err(anyhow!("Failed to register window class: {:?}", err));
            }
        }

        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            class_name,
            w!("QuickDraw Raw Input"),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            None,
        )?;

        let _ = hwnd_tx.send(SendHwnd(hwnd));

        let mut devices = Vec::new();

        if listen_mouse {
            devices.push(RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop Controls
                usUsage: 0x02,     // Mouse
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            });
        }
        
        if listen_keyboard {
            devices.push(RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop Controls
                usUsage: 0x06,     // Keyboard
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            });
        }

        if !devices.is_empty() {
            if let Err(e) = RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) {
                let _ = DestroyWindow(hwnd);
                return Err(anyhow!("Failed to register raw input devices: {}", e));
            }
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            if !is_running.load(Ordering::SeqCst) {
                break;
            }

            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam, &tx);
            } else if msg.message == WM_USER + 1 {
                break;
            }

            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let mut remove_devices = Vec::new();
        if listen_mouse {
            remove_devices.push(RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_REMOVE,
                hwndTarget: HWND::default(),
            });
        }
        if listen_keyboard {
            remove_devices.push(RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_REMOVE,
                hwndTarget: HWND::default(),
            });
        }
        if !remove_devices.is_empty() {
            let _ = RegisterRawInputDevices(&remove_devices, size_of::<RAWINPUTDEVICE>() as u32);
        }

        let _ = DestroyWindow(hwnd);
    }

    Ok(())
}

#[cfg(windows)]
fn process_raw_input(lparam: LPARAM, tx: &Sender<InputEvent>) {
    unsafe {
        let mut data_size: u32 = 0;
        let header_size = size_of::<RAWINPUTHEADER>() as u32;

        let handle = HRAWINPUT(lparam.0 as *mut c_void);

        let res = GetRawInputData(handle, RID_INPUT, None, &mut data_size, header_size);

        if res == u32::MAX || data_size == 0 {
            return;
        }

        let mut data: Vec<u8> = vec![0; data_size as usize];
        let res = GetRawInputData(
            handle,
            RID_INPUT,
            Some(data.as_mut_ptr() as *mut c_void),
            &mut data_size,
            header_size,
        );

        if res == u32::MAX {
            return;
        }

        let raw_input = &*(data.as_ptr() as *const RAWINPUT);

        // RIM_TYPEMOUSE = 0
        if raw_input.header.dwType == 0 {
            let mouse = &raw_input.data.mouse;

            // MOUSE_MOVE_RELATIVE is 0, MOUSE_MOVE_ABSOLUTE is 1. Check bit 0.
            if mouse.usFlags.0 & 0x01 == 0 {
                if mouse.lLastX != 0 || mouse.lLastY != 0 {
                    let event = InputEvent {
                        event_type: InputEventType::MouseMove {
                            dx: mouse.lLastX,
                            dy: mouse.lLastY,
                        },
                        timestamp: get_timestamp(),
                    };
                    let _ = tx.blocking_send(event);
                }
            }

            let buttons = mouse.Anonymous.Anonymous.usButtonFlags as u32;

            let process_button = |flag_down: u32, flag_up: u32, btn: MouseButton| {
                if buttons & flag_down != 0 {
                    let _ = tx.blocking_send(InputEvent {
                        event_type: InputEventType::MouseButton {
                            button: btn.clone(),
                            pressed: true,
                        },
                        timestamp: get_timestamp(),
                    });
                }
                if buttons & flag_up != 0 {
                    let _ = tx.blocking_send(InputEvent {
                        event_type: InputEventType::MouseButton {
                            button: btn,
                            pressed: false,
                        },
                        timestamp: get_timestamp(),
                    });
                }
            };

            process_button(0x0001, 0x0002, MouseButton::Left);
            process_button(0x0004, 0x0008, MouseButton::Right);
            process_button(0x0010, 0x0020, MouseButton::Middle);
            process_button(0x0040, 0x0080, MouseButton::X1);
            process_button(0x0100, 0x0200, MouseButton::X2);
        } else if raw_input.header.dwType == 1 {
            // RIM_TYPEKEYBOARD = 1
            let kb = &raw_input.data.keyboard;
            
            // Flags bit 0: RI_KEY_BREAK (0 = down, 1 = up)
            let pressed = (kb.Flags & 0x01) == 0;
            let vkey = kb.VKey;

            // Map standard virtual keys back to our string representation
            let key_str = match vkey {
                0x08 => "Backspace".to_string(),
                0x09 => "Tab".to_string(),
                0x0D => "Enter".to_string(),
                0x10 | 0xA0 | 0xA1 => "Shift".to_string(),
                0x11 | 0xA2 | 0xA3 => "Ctrl".to_string(),
                0x12 | 0xA4 | 0xA5 => "Alt".to_string(),
                0x13 => "Pause".to_string(),
                0x14 => "CapsLock".to_string(),
                0x1B => "Esc".to_string(),
                0x20 => "Space".to_string(),
                0x21 => "PageUp".to_string(),
                0x22 => "PageDown".to_string(),
                0x23 => "End".to_string(),
                0x24 => "Home".to_string(),
                0x25 => "Left".to_string(),
                0x26 => "Up".to_string(),
                0x27 => "Right".to_string(),
                0x28 => "Down".to_string(),
                0x2D => "Insert".to_string(),
                0x2E => "Delete".to_string(),
                0x30..=0x39 => ((vkey - 0x30 + b'0' as u16) as u8 as char).to_string(),
                0x41..=0x5A => ((vkey - 0x41 + b'A' as u16) as u8 as char).to_string(),
                0x5B | 0x5C => "Win".to_string(),
                0x60..=0x69 => format!("Num{}", vkey - 0x60),
                0x70..=0x87 => format!("F{}", vkey - 0x70 + 1),
                0x90 => "NumLock".to_string(),
                0x91 => "ScrollLock".to_string(),
                _ => format!("VK_{:02X}", vkey),
            };

            let _ = tx.blocking_send(InputEvent {
                event_type: InputEventType::KeyboardKey {
                    key: VirtualKey(key_str),
                    pressed,
                },
                timestamp: get_timestamp(),
            });
        }
    }
}

fn get_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
