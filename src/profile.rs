use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::collect::memory::format_bytes;
use crate::collect::process;
use crate::collect::SystemSnapshot;

#[derive(Clone, Debug)]
pub enum ProfileState {
    Idle,
    Loading,
    Ready(String),
    Error(String),
}

pub fn build_prompt(snap: &SystemSnapshot) -> String {
    let mut out = String::new();

    // System overview
    out.push_str(&format!("Host: {}\n", snap.hostname));
    out.push_str(&format!("Uptime: {}s\n", snap.uptime));
    out.push_str(&format!("CPU: {:.1}% ({} cores)\n", snap.cpu.aggregate, snap.cpu.per_core.len()));
    out.push_str(&format!(
        "Memory: {} / {} ({:.1}%)\n",
        format_bytes(snap.memory.used),
        format_bytes(snap.memory.total),
        snap.memory.ram_percent()
    ));

    for gpu in &snap.gpus {
        out.push_str(&format!("GPU: {} — {:.1}% util", gpu.name, gpu.utilization));
        if let Some(t) = gpu.temperature {
            out.push_str(&format!(", {t:.0}°C"));
        }
        if gpu.vram_total > 0 {
            out.push_str(&format!(", VRAM {}/{}", format_bytes(gpu.vram_used), format_bytes(gpu.vram_total)));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "Network: ↓{:.0} B/s  ↑{:.0} B/s\n",
        snap.network.rx_bytes_sec, snap.network.tx_bytes_sec
    ));
    if let Some(w) = snap.power.system_watts {
        out.push_str(&format!("Power: {w:.1}W\n"));
    }

    // Top 50 aggregated processes
    out.push_str("\nTop processes (grouped by name):\n");
    let mut groups = process::aggregate(&snap.processes);
    process::sort_groups(&mut groups, process::SortBy::Cpu);
    for g in groups.iter().take(50) {
        let gpu_str = if g.total_gpu_mem > 0 {
            format!(" GPU:{}", format_bytes(g.total_gpu_mem))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "  {name} (×{count}) — CPU:{cpu:.1}% MEM:{mem}{gpu}\n",
            name = g.name,
            count = g.count,
            cpu = g.total_cpu,
            mem = format_bytes(g.total_memory),
            gpu = gpu_str,
        ));
    }

    out
}

pub async fn analyze(snap: &SystemSnapshot) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY not set — export it to use profile mode")?;

    let system_stats = build_prompt(snap);

    let body = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "messages": [{
            "role": "user",
            "content": format!(
                "You are a system profiler. Analyze this snapshot and explain what's running, \
                 what's using the most resources, and anything notable or unusual. Be concise \
                 and practical — this is shown in a terminal UI.\n\n{system_stats}"
            )
        }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("failed to reach Claude API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("API error {status}: {text}");
    }

    let json: serde_json::Value = resp.json().await.context("invalid API response")?;
    let text = json["content"][0]["text"]
        .as_str()
        .unwrap_or("(empty response)")
        .to_string();

    Ok(text)
}
