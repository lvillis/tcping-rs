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
| **Pure Rust**                | No runtime dependencies, produces a tiny static binary                    |
| **ICMP-free**                | Works where traditional `ping` is blocked; relies solely on TCP handshake |
| **Cross-platform**           | Linux, macOS, Windows, *BSD, and any Tier-1 Rust target                   |
| **Continuous / Burst modes** | `-t` for continuous, `-c` for specific count, plus `-e` early exit        |
| **Machine-readable output**  | JSON / CSV via `-o`, ideal for scripts & monitoring                       |
| **Jitter stats**             | `-j` flag shows latency variance (p95)                                    |
| **Docker image**             | Multi-arch (`amd64` / `arm64`) for pipelines or Kubernetes Jobs           |


## Usage

```bash
tcping <host:port> [-c count] [-t] [-e] [-j] [-o mode]
```

Where:

- `host:port` is the host and port to ping
- `-c count` specifies the number of times to ping the host (default: 4)
- `-t` enables continuous pinging
- `-e` exits immediately after a successful probe
- `-j` calculates and displays jitter
- `-o mode` sets the output mode (`normal`, `json`, `csv`)
- `-h` displays help
- `-V` displays version

## Example

```bash
$ tcping github.com:443

Resolved address in 0.9340 ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.7510ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.4270ms
Probing 140.82.113.4:443/tcp - Port is open - time=11.4410ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.7510ms

--- 140.82.113.4:443 tcping statistics ---
4 probes sent, 4 successful, 0.00% packet loss
Round-trip min/avg/max = 11.4410ms/12.3425ms/12.7510ms
Address resolved in 0.9340 ms
```

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

## About

This tool allows you to measure the latency to a server using TCP. It is built with Rust and uses the clap library for
command line argument parsing.
