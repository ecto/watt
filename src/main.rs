mod collect;
mod config;
mod profile;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use tokio::sync::mpsc as tokio_mpsc;

use crate::config::Cli;
use crate::profile::ProfileState;
use crate::ui::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let interval = cli.interval_duration();
    let auto_profile = cli.profile;

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

    // Profile result channel
    let (profile_tx, mut profile_rx) = tokio_mpsc::unbounded_channel::<ProfileState>();

    // Terminal
    let mut terminal = ui::setup_terminal()?;
    let mut app = App::new();

    // Input stream
    let mut event_stream = EventStream::new();

    let mut first_snapshot = true;

    loop {
        tokio::select! {
            Some(snap) = snap_tokio_rx.recv() => {
                app.update(snap);

                // Auto-profile on first snapshot if --profile flag
                if first_snapshot && auto_profile {
                    first_snapshot = false;
                    app.trigger_profile();
                } else {
                    first_snapshot = false;
                }

                // Check if profile was requested
                if app.profile_requested {
                    app.profile_requested = false;
                    if let Some(snap) = &app.snapshot {
                        let snap_clone = snap.clone();
                        let tx = profile_tx.clone();
                        tokio::spawn(async move {
                            let result = match profile::analyze(&snap_clone).await {
                                Ok(text) => ProfileState::Ready(text),
                                Err(e) => ProfileState::Error(format!("{e:#}")),
                            };
                            let _ = tx.send(result);
                        });
                    }
                }

                terminal.draw(|f| ui::draw(f, &mut app))?;
            }
            Some(Ok(evt)) = event_stream.next() => {
                match evt {
                    Event::Key(key) => {
                        app.on_key(key);
                        if app.should_quit {
                            break;
                        }

                        // Check if profile was requested by keypress
                        if app.profile_requested {
                            app.profile_requested = false;
                            if let Some(snap) = &app.snapshot {
                                let snap_clone = snap.clone();
                                let tx = profile_tx.clone();
                                tokio::spawn(async move {
                                    let result = match profile::analyze(&snap_clone).await {
                                        Ok(text) => ProfileState::Ready(text),
                                        Err(e) => ProfileState::Error(format!("{e:#}")),
                                    };
                                    let _ = tx.send(result);
                                });
                            }
                        }

                        terminal.draw(|f| ui::draw(f, &mut app))?;
                    }
                    Event::Mouse(me) => {
                        let before = (app.view, app.selected_metric, app.sort_by, app.proc_scroll);
                        app.on_mouse(me);
                        if app.should_quit {
                            break;
                        }
                        let after = (app.view, app.selected_metric, app.sort_by, app.proc_scroll);
                        if before != after {
                            terminal.draw(|f| ui::draw(f, &mut app))?;
                        }
                    }
                    _ => {}
                }
            }
            Some(result) = profile_rx.recv() => {
                app.profile_state = result;
                terminal.draw(|f| ui::draw(f, &mut app))?;
            }
        }
    }

    ui::restore_terminal(&mut terminal)?;
    Ok(())
}
