// SSRF guard for user-supplied URLs handed to server-side fetchers.
use std::net::IpAddr;

use url::{Host, Url};

use crate::error::ParocheError;

fn validation(message: &str) -> ParocheError {
    ParocheError::Validation {
        message: message.to_string(),
    }
}

/// Validate a user-supplied download URL before it reaches any server-side
/// fetcher: http(s) or magnet scheme only, and no reachable host may point
/// at loopback, link-local, private, or otherwise non-public address space.
///
/// WHY: http(s) hostnames are resolved and every resolved address is
/// checked — a public-looking name can point at internal infrastructure.
/// Resolution failure rejects (fail-closed): an unresolvable host cannot be
/// fetched anyway, and uncertainty must not admit a request. Magnet URIs
/// have no direct host; their `tr` (tracker) parameters are the dialable
/// surface and are each validated instead.
pub async fn validate_download_url(raw: &str) -> Result<(), ParocheError> {
    let parsed = Url::parse(raw).map_err(|_| validation("download_url is not a valid URL"))?;

    match parsed.scheme() {
        "http" | "https" => validate_fetch_host(&parsed).await,
        "magnet" => validate_magnet_trackers(&parsed).await,
        _ => Err(validation(
            "download_url scheme must be http, https, or magnet",
        )),
    }
}

// WARNING: enqueue-time validation only. There is currently no server-side
// HTTP fetcher for download_url (downloads are BitTorrent/usenet P2P), so the
// DNS-rebinding TOCTOU is latent. Any FUTURE feature that fetches a
// download_url over HTTP server-side MUST pin the validated IP (reqwest
// resolve override) or re-validate at connect time — a host that resolves
// public here can rebind to internal space before the fetch.
async fn validate_fetch_host(parsed: &Url) -> Result<(), ParocheError> {
    let host = parsed
        .host()
        .ok_or_else(|| validation("download_url must have a host"))?;

    match host {
        Host::Ipv4(ip) => reject_disallowed_ip(IpAddr::V4(ip)),
        Host::Ipv6(ip) => reject_disallowed_ip(IpAddr::V6(ip)),
        Host::Domain(domain) => {
            // WHY: port only satisfies lookup_host's addr format; http/https always
            // have a known default so the fallback is unreachable.
            let port = parsed.port_or_known_default().unwrap_or(443);
            let addrs = tokio::net::lookup_host((domain, port))
                .await
                .map_err(|_| validation("download_url host did not resolve"))?;
            let mut resolved_any = false;
            for addr in addrs {
                resolved_any = true;
                reject_disallowed_ip(addr.ip())?;
            }
            if !resolved_any {
                return Err(validation("download_url host did not resolve"));
            }
            Ok(())
        }
    }
}

// WHY: tracker hostnames ARE DNS-resolved and checked against internal ranges;
// unresolvable trackers are allowed (auxiliary third-party endpoints).
async fn validate_magnet_trackers(parsed: &Url) -> Result<(), ParocheError> {
    for (key, value) in parsed.query_pairs() {
        if key != "tr" && !key.starts_with("tr.") {
            continue;
        }
        let tracker = Url::parse(&value)
            .map_err(|_| validation("magnet tracker parameter is not a valid URL"))?;
        match tracker.scheme() {
            "http" | "https" | "udp" | "ws" | "wss" => {}
            _ => return Err(validation("magnet tracker scheme is not allowed")),
        }
        match tracker.host() {
            Some(Host::Ipv4(ip)) => reject_disallowed_ip(IpAddr::V4(ip))?,
            Some(Host::Ipv6(ip)) => reject_disallowed_ip(IpAddr::V6(ip))?,
            Some(Host::Domain(domain)) => {
                // WHY: non-special schemes (udp) get opaque host parsing — an
                // IP literal arrives here as a Domain string, so parse it back.
                if let Ok(ip) = domain.parse::<IpAddr>() {
                    reject_disallowed_ip(ip)?;
                }
                let lower = domain.to_ascii_lowercase();
                if lower == "localhost" || lower.ends_with(".localhost") {
                    return Err(validation(
                        "magnet tracker host resolves to a private or local address",
                    ));
                }
                // WHY: resolve tracker domains and reject any pointing at internal space —
                // a tracker announce is a live server-side request, so a domain resolving
                // to loopback/private/link-local is a blind-SSRF primitive. Unresolvable
                // trackers are ALLOWED: unlike the http fetch host, a tracker is auxiliary
                // and third-party, so a dead tracker must not reject an otherwise-valid
                // magnet. NOTE: DNS rebinding is NOT closed here — the torrent client
                // re-resolves at announce time; this catches the common static-internal
                // tracker case.
                let port = tracker.port_or_known_default().unwrap_or(80);
                if let Ok(addrs) = tokio::net::lookup_host((domain, port)).await {
                    for addr in addrs {
                        reject_disallowed_ip(addr.ip())?;
                    }
                }
            }
            None => return Err(validation("magnet tracker must have a host")),
        }
    }
    Ok(())
}

fn reject_disallowed_ip(ip: IpAddr) -> Result<(), ParocheError> {
    if ip_is_disallowed(ip) {
        return Err(validation(
            "download_url host resolves to a private or local address",
        ));
    }
    Ok(())
}

fn ip_is_disallowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                // NOTE: rejects RFC 6598 shared address space (CGNAT).
                // pii-allow: 100.64.0.0/10 is the range base and prefix RFC 6598 defines, not a fleet host.
                || (u32::from(v4) & 0xFFC0_0000) == 0x6440_0000
        }
        IpAddr::V6(v6) => {
            // WHY: an IPv4-mapped IPv6 literal (::ffff:127.0.0.1) must be judged by
            // its embedded IPv4 address or it bypasses every v4 range check.
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return ip_is_disallowed(IpAddr::V4(mapped));
            }
            // WHY: checked before the ::/96 embed below so :: (unspecified) and ::1
            // (loopback) are judged as v6, not as the compatible 0.0.0.0 / 0.0.0.1.
            if v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
            {
                return true;
            }
            // WHY: deprecated IPv4-compatible IPv6 (::a.b.c.d, ::/96) embeds a v4
            // address in the low 32 bits — ::127.0.0.1 must be judged by that v4 or it
            // reaches loopback unchecked. to_ipv4_mapped only matches ::ffff:*.
            let [s0, s1, s2, s3, s4, s5, s6, s7] = v6.segments();
            if [s0, s1, s2, s3, s4, s5] == [0, 0, 0, 0, 0, 0] {
                let embedded = std::net::Ipv4Addr::from((u32::from(s6) << 16) | u32::from(s7));
                return ip_is_disallowed(IpAddr::V4(embedded));
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[tokio::test]
    async fn rejects_unparseable_url() {
        assert!(validate_download_url("not a url").await.is_err());
        assert!(validate_download_url("").await.is_err());
    }

    #[tokio::test]
    async fn rejects_non_http_schemes() {
        for url in [
            "ftp://example.com/file.torrent",
            "file:///etc/passwd",
            "gopher://example.com/",
            "javascript:alert(1)",
        ] {
            assert!(validate_download_url(url).await.is_err(), "allowed: {url}");
        }
    }

    #[tokio::test]
    async fn rejects_loopback_and_private_ip_literals() {
        for url in [
            "http://127.0.0.1/x",
            "http://127.8.9.10:8080/x",
            "https://10.0.0.1/x",
            "http://172.16.5.5/x",
            "http://192.168.1.10/x",
            "http://169.254.169.254/latest/meta-data/",
            "http://0.0.0.0/x",
            "http://100.64.0.1/x",
            "http://[::1]/x",
            "http://[fc00::1]/x",
            "http://[fe80::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://[::ffff:192.168.1.1]/x",
            "http://[::127.0.0.1]/x",
        ] {
            assert!(validate_download_url(url).await.is_err(), "allowed: {url}");
        }
    }

    #[tokio::test]
    async fn rejects_localhost_hostname() {
        assert!(validate_download_url("http://localhost/x").await.is_err());
        assert!(
            validate_download_url("http://localhost:8080/x")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn accepts_plain_magnet_uri() {
        assert!(
            validate_download_url("magnet:?xt=urn:btih:abc123def456")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn accepts_magnet_with_public_trackers() {
        assert!(
            validate_download_url(
                "magnet:?xt=urn:btih:abc123&tr=udp%3A%2F%2Ftracker.example.org%3A1337%2Fannounce&tr=https%3A%2F%2Ftracker.example.net%2Fannounce"
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn accepts_magnet_with_unresolvable_tracker() {
        // NOTE: RFC 6761 reserves .invalid — it never resolves, so the tracker
        // is allowed (auxiliary endpoint) and the magnet accepted.
        assert!(
            validate_download_url(
                "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2Fnonexistent.invalid%2Fannounce"
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn rejects_magnet_with_private_or_local_trackers() {
        for url in [
            "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2F127.0.0.1%3A8080%2Fannounce",
            "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2F192.168.1.5%2Fannounce",
            "magnet:?xt=urn:btih:abc&tr=udp%3A%2F%2F10.0.0.1%3A1337%2Fannounce",
            "magnet:?xt=urn:btih:abc&tr=http%3A%2F%2Flocalhost%3A9000%2Fannounce",
            "magnet:?xt=urn:btih:abc&tr=ftp%3A%2F%2Ftracker.example.org%2Fannounce",
        ] {
            assert!(validate_download_url(url).await.is_err(), "allowed: {url}");
        }
    }

    #[tokio::test]
    async fn accepts_public_ip_literal() {
        // NOTE: TEST-NET-3 documentation range — public per the enforced ranges,
        // never actually fetched by this validation.
        assert!(
            validate_download_url("http://203.0.113.10/file.torrent")
                .await
                .is_ok()
        );
        assert!(
            validate_download_url("https://203.0.113.10:8443/file.nzb")
                .await
                .is_ok()
        );
    }

    #[test]
    fn ip_range_classification() {
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(172, 31, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(100, 127, 0, 1))));
        assert!(ip_is_disallowed(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        // NOTE: deprecated IPv4-compatible IPv6 (::/96) — judged by the embedded v4.
        assert!(ip_is_disallowed("::127.0.0.1".parse().unwrap()));
        assert!(ip_is_disallowed("::10.0.0.1".parse().unwrap()));
        assert!(ip_is_disallowed("::192.168.1.1".parse().unwrap()));
        assert!(ip_is_disallowed("::169.254.169.254".parse().unwrap()));
        assert!(!ip_is_disallowed("::203.0.113.1".parse().unwrap()));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1))));
        assert!(!ip_is_disallowed(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
    }
}
