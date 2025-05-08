//! Basic flag parsing tests.

use clap::Parser;
use std::net::ToSocketAddrs;
use tcping::cli::{Args, OutputMode};

#[test]
fn parse_basic() {
    let a = Args::parse_from(["tcping", "127.0.0.1:80", "-c", "5"]);
    assert_eq!(a.address, "127.0.0.1:80");
    assert_eq!(a.count, 5);
    assert!(!a.continuous);
    assert_eq!(a.output_mode, OutputMode::Normal);
}

#[test]
fn continuous_flag() {
    let a = Args::parse_from(["tcping", "127.0.0.1:80", "-t"]);
    assert!(a.continuous);
}

#[test]
fn resolve_domain() {
    assert!("example.com:80".to_socket_addrs().is_ok());
}

#[test]
fn output_mode_json() {
    let a = Args::parse_from(["tcping", "127.0.0.1:80", "-o", "json"]);
    assert_eq!(a.output_mode, OutputMode::Json);
}

#[test]
fn exit_on_success() {
    let a = Args::parse_from(["tcping", "127.0.0.1:80", "-e"]);
    assert!(a.exit_on_success);
}
