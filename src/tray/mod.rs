use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, Icon,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum TrayCommand {
    Quit,
    OpenConfig,
}

pub fn start_tray(cmd_tx: mpsc::Sender<TrayCommand>) -> Result<()> {
    let icon = create_placeholder_icon();

    let configure_item = MenuItem::new("Configure...", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let tray_menu = Menu::new();
    tray_menu.append_items(&[
        &configure_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])?;

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("QuickDraw")
        .with_icon(icon)
        .build()?;

    // On Windows, we need a message loop for the tray icon and menu to work.
    // Since this is a dedicated thread, we can run our own loop.
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, DispatchMessageW, MSG, TranslateMessage};
        
        let mut msg = MSG::default();
        unsafe {
            // GetMessageW blocks until a message is available
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // After processing messages, check for menu events.
                // muda (used by tray-icon) sends events to this receiver.
                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == configure_item.id() {
                        let _ = cmd_tx.blocking_send(TrayCommand::OpenConfig);
                    } else if event.id == quit_item.id() {
                        let _ = cmd_tx.blocking_send(TrayCommand::Quit);
                    }
                }
            }
        }
    }

    Ok(())
}

fn create_placeholder_icon() -> Icon {
    let width = 16;
    let height = 16;
    // 16x16 white square (RGBA)
    let rgba = vec![255u8; (width * height * 4) as usize];
    Icon::from_rgba(rgba, width, height).expect("Failed to create placeholder icon")
}
