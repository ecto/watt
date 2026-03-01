mod collect;
mod config;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use tokio::sync::mpsc as tokio_mpsc;

use crate::config::Cli;
use crate::ui::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let interval = cli.interval_duration();

    // Spawn collector on OS thread
    let (snap_rx, _collector_handle) = collect::spawn_collector(interval);

    // Bridge std::sync::mpsc → tokio channel
    let (snap_tx, mut snap_tokio_rx) = tokio_mpsc::unbounded_channel();
    std::thread::spawn(move || {
        while let Ok(snap) = snap_rx.recv() {
            if snap_tx.send(snap).is_err() {
                break;
            }
        }
    });

    // Terminal
    let mut terminal = ui::setup_terminal()?;
    let mut app = App::new();

    // Input stream
    let mut event_stream = EventStream::new();

    loop {
        tokio::select! {
            Some(snap) = snap_tokio_rx.recv() => {
                app.update(snap);
                terminal.draw(|f| ui::draw(f, &app))?;
            }
            Some(Ok(evt)) = event_stream.next() => {
                if let Event::Key(key) = evt {
                    app.on_key(key);
                    if app.should_quit {
                        break;
                    }
                    terminal.draw(|f| ui::draw(f, &app))?;
                }
            }
        }
    }

    ui::restore_terminal(&mut terminal)?;
    Ok(())
}
