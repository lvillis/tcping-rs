<!-- ─── Language Switch & ToC (top-right) ─────────────────────────── -->
<div align="right">

<span style="color:#999;">🇺🇸 English</span> ·
<a href="README.zh-CN.md">🇨🇳 中文</a> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; Table of Contents ↗️

</div>

<h1 align="center"><code>tcping-rs</code></h1>

<p align=center>🛠️ tcping-rs: Rust (rs) TCP Ping (tcping) Utility for Port Reachability.</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/tcping.svg)](https://crates.io/crates/tcping)&nbsp;
[![Repo Size](https://img.shields.io/github/repo-size/lvillis/tcping-rs?color=328657)](https://github.com/lvillis/tcping-rs)&nbsp;
[![CI](https://github.com/lvillis/tcping-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lvillis/tcping-rs/actions)&nbsp;
[![Docker Pulls](https://img.shields.io/docker/pulls/lvillis/tcping?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Image Size](https://img.shields.io/docker/image-size/lvillis/tcping/latest?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Say Thanks](https://img.shields.io/badge/Say%20Thanks-!-1EAEDB.svg)](mailto:lvillis@outlook.com?subject=Thanks%20for%20tcping-rs!)

</div>

---

## ✨ Features

| Feature                      | Description                                                               |
|------------------------------|---------------------------------------------------------------------------|
| **Pure Rust**                | Single binary with no external runtime dependencies                       |
| **ICMP-free**                | Works where traditional `ping` is blocked; relies solely on TCP handshake |
| **Cross-platform**           | Linux, macOS, Windows, *BSD, and any Tier-1 Rust target                   |
| **Continuous / Burst modes** | `-t` for continuous, `-c` for specific count, plus `-e` early exit        |
| **Machine-readable output**  | JSON (NDJSON) / CSV / Markdown via `-o`, ideal for scripts & monitoring   |
| **Library API**              | Structured Rust API for embedding probes without parsing stdout           |
| **Jitter stats**             | `-j` prints per-probe jitter and a p95 summary                            |
| **Docker image**             | Multi-arch (`amd64` / `arm64`) for pipelines or Kubernetes Jobs           |


## Usage

```bash
tcping <host:port> [-c count] [-t] [-e] [-j] [-o mode] [--timestamp[=format] | -D] [--timeout-ms ms]
```

Where:

- `host:port` is the host and port to ping
- `-c count` specifies the number of times to ping the host (default: 4)
- `-t` enables continuous pinging
- `-e` exits immediately after a successful probe
- `-j` enables jitter output (per-probe + p95 in summary)
- `-o mode` sets the output mode (`normal`, `json`, `csv`, `md`, `color`)
- `--timestamp[=format]` emits timestamps with every probe and summary record; defaults to `iso8601`, and `--date` is an alias
- `-D` is shorthand for `--timestamp unix`
- `--timeout-ms` sets per-probe timeout in milliseconds (default: 2000)
- `-h` displays help
- `-V` displays version

## Example

```bash
$ tcping github.com:443

Resolved github.com -> 140.82.113.4  (DNS system default)  in 0.9340 ms

Probing 140.82.113.4:443/tcp - open - 12.7510 ms
Probing 140.82.113.4:443/tcp - open - 12.4270 ms
Probing 140.82.113.4:443/tcp - open - 11.4410 ms
Probing 140.82.113.4:443/tcp - open - 12.7510 ms

--- 140.82.113.4:443 tcping statistics ---
4 probes sent, 4 successful, 0.00% packet loss
Round-trip min/avg/max = 11.4410/12.3425/12.7510 ms
```

```bash
$ tcping github.com:443 --timestamp -c 2

Resolved github.com -> 140.82.113.4  (DNS system default)  in 0.9340 ms

[2026-04-08T01:15:57.952Z] Probing 140.82.113.4:443/tcp - open - 12.7510 ms
[2026-04-08T01:15:58.954Z] Probing 140.82.113.4:443/tcp - open - 12.4270 ms

[2026-04-08T01:15:58.954Z] --- 140.82.113.4:443 tcping statistics ---
2 probes sent, 2 successful, 0.00% packet loss
Round-trip min/avg/max = 12.4270/12.5890/12.7510 ms
```

## Output formats

- `-o json`: NDJSON (one JSON object per line) with `schema=tcping.v1` and `record=probe|summary`
- `-o csv`: single CSV stream with a header row, with `schema=tcping.v1` and `record=probe|summary`
- When `--timestamp` or `-D` is enabled, JSON and CSV upgrade to `schema=tcping.v2` and add `timestamp` (RFC 3339 UTC) plus `timestamp_unix_ms` fields to every `probe` and `summary` record
- Human-oriented outputs (`normal`, `color`, `md`) use the requested style directly: `iso8601` renders RFC 3339 UTC with millisecond precision, `unix` renders `seconds.millis`

## Library API

`tcping-rs` can also be used as a Rust library. The library API does not write to stdout; it returns structured session data or emits structured events for long-running probes.

For a minimal library dependency without the CLI-only `clap` / `serde_json` stack, disable default features:

```toml
[dependencies]
tcping = { version = "1", default-features = false }
```

```rust
use std::time::Duration;
use tcping::{PingOptions, Target, run_collect_async};

async fn example() -> tcping::Result<()> {
    let target = Target::parse("github.com:443")?;
    let options = PingOptions::new(target)
        .with_count(4)?
        .with_timeout(Duration::from_millis(2_000))
        .jitter(true)
        .timestamps(true);

    let session = run_collect_async(options).await?;
    println!("{:?}", session.summary);
    Ok(())
}
```

For continuous monitoring, use `run_with_handler_async` or `run_with_handler_until` and consume `PingEvent::Probe` / `PingEvent::Summary` as they are produced.

Features:

- `cli` (enabled by default): builds the `tcping` binary and enables CLI output dependencies
- `serde`: derives `Serialize` for structured library records

## Installation

### Download from Releases

Download the precompiled binaries from the [Releases Page](https://github.com/lvillis/tcping-rs/releases).

* Navigate to the [Releases](https://github.com/lvillis/tcping-rs/releases) section.
* Download the appropriate binary for your operating system.
* Extract the executable and place it in a directory included in your PATH.

### Using Docker

Run `tcping-rs` using the Docker image:

```shell
docker run --rm docker.io/lvillis/tcping:latest <host:port> [options]
```

### Using Nix

`tcping-rs` is available in nixpkgs. To install it system wide:

```nix
  environment.systemPackages = [
    pkgs.tcping-rs
  ];
```

Or spawn in a nix-shell:

```console
nix-shell -p tcping-rs
```

## About

This tool allows you to measure the latency to a server using TCP. It is built with Rust and uses the clap library for
command line argument parsing, plus Tokio for the async loop and timers.
