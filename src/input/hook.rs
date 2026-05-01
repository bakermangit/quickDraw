#[cfg(windows)]
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
#[cfg(windows)]
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use anyhow::{anyhow, Result};
use tokio::sync::mpsc;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    GetCursorPos, HHOOK, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_QUIT,
    WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use crate::types::{InputEvent, InputEventType, MouseButton};
use super::InputSource;

#[cfg(windows)]
static EVENT_TX: Mutex<Option<mpsc::Sender<InputEvent>>> = Mutex::new(None);
#[cfg(windows)]
static SHOULD_BLOCK: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static LAST_X: AtomicI32 = AtomicI32::new(0);
#[cfg(windows)]
static LAST_Y: AtomicI32 = AtomicI32::new(0);

#[cfg(windows)]
struct SendHhook(HHOOK);
#[cfg(windows)]
unsafe impl Send for SendHhook {}
#[cfg(windows)]
unsafe impl Sync for SendHhook {}

pub struct HookInputSource {
    thread_handle: Option<JoinHandle<()>>,
    thread_id: Option<u32>,
}

impl HookInputSource {
    pub fn new() -> Self {
        Self {
            thread_handle: None,
            thread_id: None,
        }
    }

    #[allow(dead_code)]
    pub fn set_block(&self, _block: bool) {
        #[cfg(windows)]
        SHOULD_BLOCK.store(_block, Ordering::Relaxed);
    }
}

impl InputSource for HookInputSource {
    fn start(&mut self, _tx: mpsc::Sender<InputEvent>) -> Result<()> {
        #[cfg(windows)]
        {
            if self.thread_handle.is_some() {
                return Err(anyhow!("HookInputSource is already running"));
            }

            {
                let mut guard = EVENT_TX.lock().unwrap();
                *guard = Some(_tx);
            }

            let (id_tx, id_rx) = std::sync::mpsc::channel();

            let handle = thread::spawn(move || {
                unsafe {
                    let thread_id = GetCurrentThreadId();

                    let mut pos = windows::Win32::Foundation::POINT::default();
                    let _ = GetCursorPos(&mut pos);
                    LAST_X.store(pos.x, Ordering::Relaxed);
                    LAST_Y.store(pos.y, Ordering::Relaxed);

                    let h_instance = match GetModuleHandleW(None) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = id_tx.send(Err(anyhow!("Failed to get module handle: {}", e)));
                            return;
                        }
                    };

                    let hook = match SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(low_level_mouse_proc),
                        h_instance,
                        0,
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = id_tx.send(Err(anyhow!("Failed to install mouse hook: {}", e)));
                            return;
                        }
                    };

                    let _ = id_tx.send(Ok(thread_id));

                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                        if msg.message == WM_QUIT {
                            break;
                        }
                    }

                    let _ = UnhookWindowsHookEx(hook);
                }
            });

            self.thread_handle = Some(handle);
            let thread_id = id_rx.recv().map_err(|e| anyhow!("Failed to receive thread ID: {}", e))??;
            self.thread_id = Some(thread_id);

            Ok(())
        }
        #[cfg(not(windows))]
        Err(anyhow!("HookInputSource is only supported on Windows"))
    }

    fn stop(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            if let Some(tid) = self.thread_id.take() {
                unsafe {
                    let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }

            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }

            Ok(())
        }
        #[cfg(not(windows))]
        Ok(())
    }

    fn can_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "hook"
    }
}

impl Drop for HookInputSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(windows)]
unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let ms = *(lparam.0 as *const MSLLHOOKSTRUCT);

        let event_type = match wparam.0 as u32 {
            WM_MOUSEMOVE => {
                let dx = ms.pt.x - LAST_X.swap(ms.pt.x, Ordering::Relaxed);
                let dy = ms.pt.y - LAST_Y.swap(ms.pt.y, Ordering::Relaxed);

                if dx == 0 && dy == 0 {
                    None
                } else {
                    Some(InputEventType::MouseMove { dx, dy })
                }
            }
            WM_LBUTTONDOWN => Some(InputEventType::MouseButton {
                button: MouseButton::Left,
                pressed: true,
            }),
            WM_LBUTTONUP => Some(InputEventType::MouseButton {
                button: MouseButton::Left,
                pressed: false,
            }),
            WM_RBUTTONDOWN => Some(InputEventType::MouseButton {
                button: MouseButton::Right,
                pressed: true,
            }),
            WM_RBUTTONUP => Some(InputEventType::MouseButton {
                button: MouseButton::Right,
                pressed: false,
            }),
            WM_MBUTTONDOWN => Some(InputEventType::MouseButton {
                button: MouseButton::Middle,
                pressed: true,
            }),
            WM_MBUTTONUP => Some(InputEventType::MouseButton {
                button: MouseButton::Middle,
                pressed: false,
            }),
            WM_XBUTTONDOWN => {
                let button = if (ms.mouseData >> 16) & 0xFFFF == 1 {
                    MouseButton::X1
                } else {
                    MouseButton::X2
                };
                Some(InputEventType::MouseButton {
                    button,
                    pressed: true,
                })
            }
            WM_XBUTTONUP => {
                let button = if (ms.mouseData >> 16) & 0xFFFF == 1 {
                    MouseButton::X1
                } else {
                    MouseButton::X2
                };
                Some(InputEventType::MouseButton {
                    button,
                    pressed: false,
                })
            }
            _ => None,
        };

        if let Some(et) = event_type {
            if let Ok(guard) = EVENT_TX.lock() {
                if let Some(tx) = guard.as_ref() {
                    let event = InputEvent {
                        event_type: et,
                        timestamp: get_timestamp(),
                    };
                    let _ = tx.blocking_send(event);
                }
            }
        }

        if SHOULD_BLOCK.load(Ordering::Relaxed) {
            return LRESULT(1);
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

fn get_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
