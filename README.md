<p align="center">
  <img src="assets/hero.webp" alt="watt" width="600"><br>
  <strong>watt</strong>
</p>

# watt

A terminal system monitor that actually knows your GPU exists.

```
⚡ watt │ macbook │ up 4h32m │ cpu 12% │ mem 61% (19.2G/32.0G) │ 12 cores
╭─────────────────────────────────────────────────────╮
│ CPU 12.3%  ▁▂▁▃▂▁▂▅▃▂▁▁▂▃▅▇█▅▃▂▁▁▂▁▁▂▁▁▂▁▁▂▁▁▁▁ │
│ MEM 61.0%  ▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇ │
│ GPU 24.1%  ▁▁▁▂▃▅▃▂▁▁▃▅▇▅▃▂▁▁▁▂▃▂▁▁▁▁▁▁▁▁▁▁▁▂▃▂ │
│ NET ↓1.2M  ▁▁▁▁▁▁▁▁▁▃▅█▅▃▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁ │
│ PWR 8.2W   ▂▂▂▃▃▂▂▃▅▃▂▂▂▂▃▅▇▅▃▂▂▂▃▂▂▃▂▂▃▂▂▂▂▂▂▂ │
╰─────────────────────────────────────────────────────╯
```

<table align="left"><tr><td align="center">
<img src="assets/wat.png" width="60"><br>
<sub><em>other monitors just<br>pretend GPUs don't exist?</em></sub>
</td></tr></table>

Real-time CPU, memory, GPU, network, disk, and power monitoring with per-core heatmaps, process grouping, and Claude-powered system analysis. Catppuccin Mocha theme. On Apple Silicon, reads GPU stats directly from the hardware — no sudo, no kernel extensions.

<br clear="both">

## Install

```
cargo install --path .
```

NVIDIA GPU support:

```
cargo install --path . --features nvidia
```

## Usage

```
watt                              # launch
watt -i 500                       # 500ms refresh interval
ANTHROPIC_API_KEY=... watt -p     # auto-analyze on startup
```

## Keybinds

| Key | Action |
|-----|--------|
| `j` `k` / `↑` `↓` | Navigate metrics / scroll |
| `Enter` | Drill into process table |
| `Esc` | Back / clear filter |
| `/` | Filter processes |
| `c` `m` `n` `p` | Sort by CPU / Memory / Name / PID |
| `g` `G` | Jump to top / bottom |
| `P` | Claude system profile |
| `q` | Quit |

## Views

**Chart** — sparkline per metric, auto-scaled. Select with j/k, drill down with Enter. Mouse hover highlights, click drills down, scroll wheel navigates.

**Processes** — grouped by name with summed metrics. 47 Chrome Helper PIDs become one row showing `Chrome Helper (×47)`. Sortable columns, filterable with `/`.

**Profile** — press `P` from any view. Sends an aggregated snapshot to Claude (requires `ANTHROPIC_API_KEY`) and displays a plain-language analysis of what's running and why. Scrollable with j/k.

## How It Works

### Apple Silicon GPU

<table align="right"><tr><td align="center">
<img src="assets/wat.png" width="60"><br>
<sub><em>no sudo? no kernel<br>extension? just vibes?</em></sub>
</td></tr></table>

macOS exposes GPU frequency residency counters through a private IOReport API — the same one Activity Monitor uses internally. watt subscribes to these counters and derives utilization from active-vs-idle time. No kernel extension, no entitlement, no sudo. Apple just left the door open.

- **Utilization** from frequency residency states (active vs idle time)
- **Power** from the Energy Model channel (millijoules → watts)
- **Temperature** from SMC sensor keys (`Tg*` prefix)

### NVIDIA

NVML via the `nvidia` feature flag. Per-process GPU memory shown in the process table.

### Process Aggregation

<table align="left"><tr><td align="center">
<img src="assets/wat.png" width="60"><br>
<sub><em>forty-seven<br>Chrome Helpers???</em></sub>
</td></tr></table>

Processes are grouped by name into `ProcessGroup` structs via a HashMap. Each group sums CPU%, memory, and GPU memory across all PIDs sharing that name. The table shows count, aggregated metrics, and a `×N` suffix.

<br clear="both">

### Claude Profile

<table align="right"><tr><td align="center">
<img src="assets/wat.png" width="60"><br>
<sub><em>it asks an AI what your<br>computer is doing?</em></sub>
</td></tr></table>

`P` or `--profile` builds a text prompt from the current snapshot — system stats plus the top 50 aggregated processes — and POSTs to the Claude Messages API (`claude-sonnet-4-20250514`, 1024 tokens). The response streams into a scrollable view. Requires `ANTHROPIC_API_KEY` env var.

## Source Layout

```
src/
├── main.rs                 tokio select loop, channel wiring
├── config.rs               CLI flags (clap)
├── profile.rs              Claude API client, prompt builder
├── collect/
│   ├── mod.rs              snapshot collector (OS thread)
│   ├── process.rs          ProcessSnapshot, ProcessGroup, aggregate
│   ├── cpu.rs              per-core CPU
│   ├── memory.rs           RAM/swap
│   ├── gpu.rs              GpuBackend trait, NVIDIA impl
│   ├── apple_gpu.rs        Apple Silicon IOReport backend
│   ├── network.rs          rx/tx bytes
│   ├── disk.rs             disk I/O
│   └── power.rs            system power draw
└── ui/
    ├── mod.rs              terminal setup, draw dispatch
    ├── app.rs              state machine, keybinds, mouse
    ├── cpu_view.rs         sparklines, gauges, core heatmap
    ├── process_view.rs     grouped process table
    ├── profile_view.rs     Claude analysis view
    ├── header.rs           status bar
    ├── sparkline.rs        ring buffer
    └── theme.rs            Catppuccin Mocha palette
```

## License

[MIT](LICENSE)
