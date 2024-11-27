<div align=right>Table of Contents↗️</div>

<h1 align=center><code>tcping-rs</code></h1>

<p align=center>🛠️ tcping-rs: Rust (rs) TCP Ping (tcping) Utility for Port Reachability.</p>

<div align=center>
  <a href="https://crates.io/crates/tcping">
    <img src="https://img.shields.io/crates/v/tcping.svg" alt="crates.io version">
  </a>
  <a href="https://crates.io/crates/tcping">
    <img src="https://img.shields.io/github/repo-size/lvillis/tcping-rs?style=flat-square&color=328657" alt="crates.io version">
  </a>
  <a href="https://github.com/lvillis/tcping-rs/actions">
    <img src="https://github.com/lvillis/tcping-rs/actions/workflows/ci.yaml/badge.svg" alt="build status">
  </a>
  <a href="https://hub.docker.com/r/lvillis/tcping">
    <img src="https://img.shields.io/docker/pulls/lvillis/tcping?style=flat-square" alt="docker pulls">
  </a>
  <a href="https://hub.docker.com/r/lvillis/tcping">
    <img src="https://img.shields.io/docker/image-size/lvillis/tcping/latest?style=flat-square" alt="image size">
  </a>
  <a href="mailto:lvillis@outlook.com?subject=Thanks%20for%20tcping-rs!">
    <img src="https://img.shields.io/badge/Say%20Thanks-!-1EAEDB.svg" alt="say thanks">
  </a>

</div>

---

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

## Special thanks

[![Jetbrains Logo](https://krwu.github.io/img/jetbrains.svg)](https://www.jetbrains.com/?from=init-rs)

Thanks to [Jetbrains](https://www.jetbrains.com/?from=tcping-rs) for supporting this small open source project!
