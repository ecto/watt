use std::time::Duration;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "watt", about = "Unified system monitor")]
pub struct Cli {
    /// Refresh interval in milliseconds
    #[arg(short, long, default_value = "100")]
    pub interval: u64,

    /// Run Claude-powered system profile analysis on startup
    #[arg(short, long)]
    pub profile: bool,
}

impl Cli {
    pub fn interval_duration(&self) -> Duration {
        Duration::from_millis(self.interval)
    }
}
