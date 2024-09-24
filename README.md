<div align=right>Table of Contents↗️</div>

<h1 align=center><code>tcping-rs</code></h1>

<p align=center>🌐 A TCP ping utility to determine reachability of a TCP port, using Rust.</p>

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
  <a href="mailto:lvillis@outlook.com?subject=Thanks%20for%20tcping-rs!">
    <img src="https://img.shields.io/badge/Say%20Thanks-!-1EAEDB.svg" alt="say thanks">
  </a>
</div>

---

## Installation

This project is built with Rust and Cargo. To install Rust and Cargo, follow the instructions [here](https://www.rust-lang.org/tools/install).

To build the project, navigate to the project directory and run:

```bash
cargo build --release
```

This will create an executable in the ./target/release directory.

## Running

To run the executable, navigate to the ./target/release directory and run:

```bash
./target/release/tcping
```

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


## About

This tool allows you to measure the latency to a server using TCP. It is built with Rust and uses the clap library for command line argument parsing.

## Special thanks

[![Jetbrains Logo](https://krwu.github.io/img/jetbrains.svg)](https://www.jetbrains.com/?from=init-rs)

Thanks to [Jetbrains](https://www.jetbrains.com/?from=tcping-rs) for supporting this small open source project!
