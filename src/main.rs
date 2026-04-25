mod config;
mod pipeline;
mod types;
mod input;
mod gesture;
mod output;
mod audio;
mod tray;
mod server;

use std::sync::Arc;
use tokio::sync::Mutex;
use server::ServerState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        if arg == "--capture" {
            let name = args.next().ok_or_else(|| anyhow::anyhow!("Missing gesture name"))?;
            let action = args.next().ok_or_else(|| anyhow::anyhow!("Missing gesture action"))?;
            
            let config = config::load_config()?;
            let (_, capture_rx) = tokio::sync::mpsc::channel(1);
            let pipeline = pipeline::build_pipeline(config, capture_rx)?;
            pipeline.capture_one(name, action).await?;
            return Ok(());
        }
    }

    println!("QuickDraw starting...");
    
    let config = config::load_config()?;
    let gesture_profile = config::load_gesture_profile(&config.general.gesture_profile)?;

    let (capture_tx, capture_rx) = tokio::sync::mpsc::channel(1);
    let pipeline = pipeline::build_pipeline(config.clone(), capture_rx)?;
    
    let shared_state = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        gesture_profile,
    }));
    
    tokio::spawn(server::start(config.server.port, shared_state, capture_tx));

    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel(8);
    std::thread::spawn(move || {
        if let Err(e) = tray::start_tray(cmd_tx) {
            tracing::error!("Tray error: {}", e);
        }
    });

    let mut pipeline_fut = Box::pin(pipeline.run());

    loop {
        tokio::select! {
            res = &mut pipeline_fut => {
                res?;
                break;
            }
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    tray::TrayCommand::Quit => {
                        tracing::info!("Quit command received from tray");
                        std::process::exit(0);
                    }
                    tray::TrayCommand::OpenConfig => {
                        tracing::info!("Opening config UI");
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "http://localhost:9876"])
                            .spawn();
                    }
                }
            }
        }
    }
    
    Ok(())
}

