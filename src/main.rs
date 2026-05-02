use std::sync::Arc;
use tokio::sync::Mutex;
use quickdraw::server::ServerState;
use quickdraw::types::SystemCommand;
use quickdraw::{config, pipeline, tray, server};

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
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel(8);

    let pipeline = pipeline::build_pipeline(config.clone(), capture_rx)?;
    
    let shared_state = Arc::new(Mutex::new(ServerState {
        config: config.clone(),
        gesture_profile,
        capture_tx,
        cmd_tx: cmd_tx.clone(),
    }));
    
    tokio::spawn(server::start(config.server.port, shared_state.clone()));

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
                    SystemCommand::Quit => {
                        tracing::info!("Quit command received");
                        std::process::exit(0);
                    }
                    SystemCommand::OpenConfig => {
                        tracing::info!("Opening config UI");
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "http://localhost:9876"])
                            .spawn();
                    }
                    SystemCommand::ReloadEngine => {
                        tracing::info!("Reloading engine...");

                        // 1. Drop the current pipeline_fut to safely shut down existing input hooks
                        drop(pipeline_fut);

                        // 2. Load fresh config and gesture profile
                        let config = config::load_config()?;
                        let gesture_profile = config::load_gesture_profile(&config.general.gesture_profile)?;

                        // 3. Create a new capture channel
                        let (capture_tx, capture_rx) = tokio::sync::mpsc::channel(1);

                        // 4. Update the ServerState
                        {
                            let mut state = shared_state.lock().await;
                            state.config = config.clone();
                            state.gesture_profile = gesture_profile;
                            state.capture_tx = capture_tx;
                        }

                        // 5. Build a new pipeline and restart it
                        let pipeline = pipeline::build_pipeline(config, capture_rx)?;
                        pipeline_fut = Box::pin(pipeline.run());

                        tracing::info!("Engine reloaded successfully.");
                    }
                }
            }
        }
    }
    
    Ok(())
}
