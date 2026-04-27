use std::sync::mpsc;
use std::thread;
use crate::config::GeneralConfig;

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::WindowsAndMessaging::*,
};

pub enum TraceCommand {
    Begin(f64, f64),
    AddPoint(f64, f64),
    End,
}

pub struct TraceOverlay {
    command_tx: mpsc::Sender<TraceCommand>,
    hwnd: isize,
}

#[cfg(windows)]
fn parse_hex_color(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    if let Ok(val) = u32::from_str_radix(hex, 16) {
        if hex.len() == 6 {
            let r = (val >> 16) & 0xFF;
            let g = (val >> 8) & 0xFF;
            let b = val & 0xFF;
            // GDI pen COLORREF is 0x00BBGGRR
            let color = r | (g << 8) | (b << 16);
            // Ensure color doesn't match our transparency key 0x010101
            if color == 0x010101 {
                return 0x0000FF00; // Default green
            }
            return color;
        }
    }
    0x0000FF00 // Default green
}

impl TraceOverlay {
    pub fn new(config: GeneralConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (hwnd_tx, hwnd_rx) = mpsc::channel();

        thread::spawn(move || {
            #[cfg(windows)]
            unsafe {
                let h_instance = GetModuleHandleW(None).unwrap_or_default();
                let window_class = w!("QuickDrawTraceOverlay");

                let wc = WNDCLASSW {
                    hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                    hInstance: h_instance.into(),
                    lpszClassName: window_class,
                    lpfnWndProc: Some(wnd_proc),
                    ..Default::default()
                };

                RegisterClassW(&wc);

                let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

                let hwnd = CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
                    window_class,
                    w!("QuickDraw Trace"),
                    WS_POPUP,
                    x,
                    y,
                    width,
                    height,
                    None,
                    None,
                    h_instance,
                    None,
                ).unwrap_or_default();

                let _ = hwnd_tx.send(hwnd.0 as isize);

                let color = parse_hex_color(&config.trace_color);
                let transparent_key = COLORREF(0x010101);

                // Setup GDI resources
                let hdc_screen = GetDC(None);
                let hdc_mem = CreateCompatibleDC(hdc_screen);

                let mut bmi = BITMAPINFO::default();
                bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = width;
                bmi.bmiHeader.biHeight = -height; // Top-down
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = 0; // BI_RGB is 0

                let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
                let h_bitmap = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
                let old_bitmap = SelectObject(hdc_mem, h_bitmap);

                let mut pen = CreatePen(PS_SOLID, 3, COLORREF(color));
                let mut old_pen = SelectObject(hdc_mem, pen);

                let mut current_stroke_width = config.trace_max_stroke as f64;

                let mut msg = MSG::default();
                loop {
                    if GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        if msg.message == WM_QUIT {
                            break;
                        }

                        if msg.message == WM_USER_WAKE {
                            while let Ok(cmd) = command_rx.try_recv() {
                                match cmd {
                                    TraceCommand::Begin(start_x, start_y) => {
                                        // Update pen for beginning
                                        if config.trace_finesse_enabled {
                                            current_stroke_width = config.trace_min_stroke as f64;
                                        } else {
                                            current_stroke_width = config.trace_max_stroke as f64;
                                        }

                                        let new_pen = CreatePen(PS_SOLID, current_stroke_width as i32, COLORREF(color));
                                        SelectObject(hdc_mem, new_pen);
                                        let _ = DeleteObject(pen);
                                        pen = new_pen;

                                        // Fill with transparency key
                                        let h_brush = CreateSolidBrush(transparent_key);
                                        let rect = RECT { left: 0, top: 0, right: width, bottom: height };
                                        let _ = FillRect(hdc_mem, &rect, h_brush);
                                        let _ = DeleteObject(h_brush);

                                        let _ = MoveToEx(hdc_mem, (start_x - x as f64) as i32, (start_y - y as f64) as i32, None);

                                        let pt_dst = POINT { x, y };
                                        let size = SIZE { cx: width, cy: height };
                                        let pt_src = POINT { x: 0, y: 0 };

                                        let _ = UpdateLayeredWindow(
                                            hwnd,
                                            hdc_screen,
                                            Some(&pt_dst),
                                            Some(&size),
                                            hdc_mem,
                                            Some(&pt_src),
                                            transparent_key,
                                            None,
                                            ULW_COLORKEY,
                                        );

                                        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                                    }
                                    TraceCommand::AddPoint(px, py) => {
                                        if config.trace_finesse_enabled && current_stroke_width < config.trace_max_stroke as f64 {
                                            current_stroke_width = (current_stroke_width + config.trace_growth_rate).min(config.trace_max_stroke as f64);

                                            let new_pen = CreatePen(PS_SOLID, current_stroke_width as i32, COLORREF(color));
                                            SelectObject(hdc_mem, new_pen);
                                            let _ = DeleteObject(pen);
                                            pen = new_pen;
                                        }

                                        let _ = LineTo(hdc_mem, (px - x as f64) as i32, (py - y as f64) as i32);

                                        let pt_src = POINT { x: 0, y: 0 };
                                        let pt_dst = POINT { x, y };
                                        let size = SIZE { cx: width, cy: height };

                                        let _ = UpdateLayeredWindow(
                                            hwnd,
                                            hdc_screen,
                                            Some(&pt_dst),
                                            Some(&size),
                                            hdc_mem,
                                            Some(&pt_src),
                                            transparent_key,
                                            None,
                                            ULW_COLORKEY,
                                        );
                                    }
                                    TraceCommand::End => {
                                        let _ = ShowWindow(hwnd, SW_HIDE);
                                    }
                                }
                            }
                        }

                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    } else {
                        break;
                    }
                }

                // Cleanup
                SelectObject(hdc_mem, old_pen);
                let _ = DeleteObject(pen);
                SelectObject(hdc_mem, old_bitmap);
                let _ = DeleteObject(h_bitmap);
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(None, hdc_screen);
            }
            #[cfg(not(windows))]
            {
                let _ = config;
                let _ = command_rx;
                let _ = hwnd_tx;
            }
        });

        let hwnd = hwnd_rx.recv().unwrap_or(0);
        Self { command_tx, hwnd }
    }

    pub fn send(&self, command: TraceCommand) {
        if self.command_tx.send(command).is_ok() {
            #[cfg(windows)]
            unsafe {
                let _ = PostMessageW(windows::Win32::UI::WindowsAndMessaging::HWND(self.hwnd as *mut _), WM_USER_WAKE, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
            }
        }
    }
}

#[cfg(windows)]
const WM_USER_WAKE: u32 = WM_USER + 1;

#[cfg(windows)]
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
