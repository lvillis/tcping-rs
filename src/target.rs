//! Target parsing and DNS resolution.

use crate::{
    error::{Result, TcpingError},
    timestamp::RecordTimestamp,
};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Instant,
};

/// User-requested TCP endpoint before DNS resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    host: String,
    port: u16,
}

impl Target {
    /// Build a target from a host and TCP port.
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self> {
        let host = host.into();
        let host = host.trim();
        if host.is_empty() {
            return Err(TcpingError::InvalidTarget("target host is empty".into()));
        }

        if port == 0 {
            return Err(TcpingError::InvalidTarget(
                "port must be an integer between 1 and 65535".into(),
            ));
        }

        Ok(Self {
            host: host.to_string(),
            port,
        })
    }

    /// Parse `<host:port>`, including bracketed IPv6 targets such as `[::1]:443`.
    pub fn parse(address: &str) -> Result<Self> {
        address.parse()
    }

    /// Hostname or IP literal.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// TCP port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether the host is already an IP literal and does not require DNS.
    pub fn is_ip_literal(&self) -> bool {
        self.host.parse::<IpAddr>().is_ok()
    }

    /// Socket address if this target is an IP literal.
    pub fn socket_addr_if_literal(&self) -> Option<SocketAddr> {
        self.host
            .parse::<IpAddr>()
            .ok()
            .map(|ip| SocketAddr::new(ip, self.port))
    }
}

impl FromStr for Target {
    type Err = TcpingError;

    fn from_str(address: &str) -> Result<Self> {
        let trimmed = address.trim();
        let (host, port) = if let Some(rest) = trimmed.strip_prefix('[') {
            let closing = rest.find(']').ok_or_else(|| {
                TcpingError::InvalidTarget("missing closing ']' in target".into())
            })?;
            let host = &rest[..closing];
            let port = rest[closing + 1..].strip_prefix(':').ok_or_else(|| {
                TcpingError::InvalidTarget("IPv6 target must end with ]:port".into())
            })?;
            (host, port)
        } else {
            trimmed.rsplit_once(':').ok_or_else(|| {
                TcpingError::InvalidTarget("target must be in the form host:port".into())
            })?
        };

        let port = port.parse::<u16>().map_err(|_| {
            TcpingError::InvalidTarget("port must be an integer between 1 and 65535".into())
        })?;

        Self::new(host, port)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') && self.is_ip_literal() {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

/// DNS-resolved endpoint used by a probing session.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTarget {
    pub target: Target,
    pub addr: SocketAddr,
    pub resolve_time_ms: f64,
    pub dns_server: Option<IpAddr>,
    pub resolved_at: RecordTimestamp,
}

/// Best-effort reader for the first DNS server listed in `/etc/resolv.conf`.
/// Returns `None` on Windows or failure.
fn first_dns_server() -> Option<IpAddr> {
    #[cfg(unix)]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open("/etc/resolv.conf").ok()?;
        for line in BufReader::new(file).lines().map_while(|line| line.ok()) {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("nameserver") {
                return rest.trim().parse().ok();
            }
        }
    }
    None
}

/// Resolve a target to the first socket address returned by the system resolver.
pub async fn resolve_target(target: &Target) -> Result<ResolvedTarget> {
    if let Some(addr) = target.socket_addr_if_literal() {
        return Ok(ResolvedTarget {
            target: target.clone(),
            addr,
            resolve_time_ms: 0.0,
            dns_server: None,
            resolved_at: RecordTimestamp::now(),
        });
    }

    let start = Instant::now();
    let mut addrs = tokio::net::lookup_host((target.host(), target.port())).await?;
    let addr = addrs.next().ok_or(TcpingError::NoAddress)?;
    let resolve_time_ms = start.elapsed().as_secs_f64() * 1_000.0;

    Ok(ResolvedTarget {
        target: target.clone(),
        addr,
        resolve_time_ms,
        dns_server: first_dns_server(),
        resolved_at: RecordTimestamp::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_literal() {
        let target = Target::parse("127.0.0.1:80").unwrap();
        assert!(target.is_ip_literal());
        assert_eq!(target.host(), "127.0.0.1");
        assert_eq!(target.port(), 80);
        assert_eq!(
            target.socket_addr_if_literal(),
            Some("127.0.0.1:80".parse().unwrap())
        );
    }

    #[test]
    fn parses_ipv6_literal() {
        let target = Target::parse("[::1]:443").unwrap();
        assert!(target.is_ip_literal());
        assert_eq!(target.host(), "::1");
        assert_eq!(target.port(), 443);
        assert_eq!(
            target.socket_addr_if_literal(),
            Some("[::1]:443".parse().unwrap())
        );
    }

    #[test]
    fn parses_hostname() {
        let target = Target::parse("example.com:9000").unwrap();
        assert!(!target.is_ip_literal());
        assert_eq!(target.host(), "example.com");
        assert_eq!(target.port(), 9000);
        assert!(target.socket_addr_if_literal().is_none());
    }

    #[test]
    fn rejects_missing_port() {
        assert!(Target::parse("example.com").is_err());
    }

    #[test]
    fn rejects_port_zero() {
        assert!(Target::parse("example.com:0").is_err());
    }
}
