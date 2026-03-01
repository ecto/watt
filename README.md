<p align="center">
  <img src="assets/hero.webp" alt="watt" width="400">
</p>

<h1 align="center">watt</h1>

<p align="center">
  <strong>a terminal system monitor that actually knows your GPU exists</strong>
</p>

<p align="center">
  <code>cargo install --path .</code>
</p>

---

You know that feeling when you open a system monitor and the GPU tab says "No GPU detected"? On your $3,000 MacBook Pro? With the chip Apple won't shut up about?

**wat.**

watt fixes that. Real-time CPU, memory, GPU, and power monitoring in your terminal. On Apple Silicon, it reads GPU utilization, power draw, and temperature directly from the hardware — no sudo, no kernel extensions, no nonsense.

## Features

- **CPU** — per-core utilization, frequency, temperature
- **Memory** — RAM and swap usage
- **GPU** — utilization %, power (watts), temperature
  - Apple Silicon via IOReport + SMC (sudoless)
  - NVIDIA via NVML (`--features nvidia`)
- **Processes** — sortable by CPU/memory usage
- **Power** — system power draw

## Install

```
cargo install --path .
```

For NVIDIA GPU support:

```
cargo install --path . --features nvidia
```

## Usage

```
watt
```

Navigate tabs with `1-5` or arrow keys. `q` to quit.

## How it works on Apple Silicon

watt uses the same private IOReport API that Apple's own Activity Monitor uses internally:

- **GPU utilization** from frequency residency states (how much time the GPU spends active vs idle)
- **GPU power** from the Energy Model channel (millijoules per sample interval, converted to watts)
- **GPU temperature** from SMC sensor keys (`Tg*` prefix)
- **Chip info** from `system_profiler` + IORegistry DVFS tables

No elevated privileges required. Just works.

## License

MIT
