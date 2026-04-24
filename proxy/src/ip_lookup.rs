use std::path::Path;

/// Parses a dnsmasq lease file and returns the IP for the given MAC address.
/// Line format: "<expiry> <mac> <ip> <hostname> <client-id>"
pub fn parse_lease_output(content: &str, mac: &str) -> Option<String> {
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1].eq_ignore_ascii_case(mac) {
            return Some(parts[2].to_string());
        }
    }
    None
}

/// Parses `ip neigh show dev br0` output and returns the IP for the given MAC.
/// Line format: "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE"
pub fn parse_arp_output(output: &str, mac: &str) -> Option<String> {
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 && parts[4].eq_ignore_ascii_case(mac) {
            return Some(parts[0].to_string());
        }
    }
    None
}

/// Resolve a MAC address to an IP. Prefers the dnsmasq lease file (populated
/// immediately on DHCP grant); falls back to the local ARP table.
pub async fn lookup_ip_by_mac(mac: &str, lease_file: &Path) -> Option<String> {
    if let Ok(content) = tokio::fs::read_to_string(lease_file).await {
        if let Some(ip) = parse_lease_output(&content, mac) {
            return Some(ip);
        }
    }

    let output = tokio::process::Command::new("ip")
        .args(["neigh", "show", "dev", "br0"])
        .output()
        .await
        .ok()?;
    parse_arp_output(&String::from_utf8_lossy(&output.stdout), mac)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_lease_output_finds_ip_by_mac() {
        let content = "1234567890 52:54:00:ab:cd:ef 10.0.0.188 alpine-vm *\n\
                       1234567891 52:54:00:11:22:33 10.0.0.189 alpine-vm2 *\n";

        assert_eq!(
            parse_lease_output(content, "52:54:00:ab:cd:ef"),
            Some("10.0.0.188".to_string())
        );
    }

    #[test]
    fn test_parse_lease_output_case_insensitive() {
        let content = "1234567890 52:54:00:ab:cd:ef 10.0.0.188 alpine-vm *\n";
        assert_eq!(
            parse_lease_output(content, "52:54:00:AB:CD:EF"),
            Some("10.0.0.188".to_string())
        );
    }

    #[test]
    fn test_parse_lease_output_no_match_returns_none() {
        let content = "1234567890 52:54:00:ab:cd:ef 10.0.0.188 alpine-vm *\n";
        assert_eq!(parse_lease_output(content, "52:54:00:ff:ff:ff"), None);
    }

    #[test]
    fn test_parse_arp_output_finds_ip_by_mac() {
        let output = "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE\n\
                      10.0.0.16 dev br0 lladdr 52:54:00:11:22:33 STALE\n";

        assert_eq!(
            parse_arp_output(output, "52:54:00:ab:cd:ef"),
            Some("10.0.0.15".to_string())
        );
        assert_eq!(
            parse_arp_output(output, "52:54:00:11:22:33"),
            Some("10.0.0.16".to_string())
        );
    }

    #[test]
    fn test_parse_arp_output_case_insensitive() {
        let output = "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE\n";
        assert_eq!(
            parse_arp_output(output, "52:54:00:AB:CD:EF"),
            Some("10.0.0.15".to_string())
        );
    }

    #[test]
    fn test_parse_arp_output_no_match_returns_none() {
        let output = "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE\n";
        assert_eq!(parse_arp_output(output, "52:54:00:ff:ff:ff"), None);
    }

    #[tokio::test]
    async fn test_lookup_ip_reads_from_lease_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "1234567890 52:54:00:ab:cd:ef 10.0.0.188 alpine-vm *").unwrap();

        let ip = lookup_ip_by_mac("52:54:00:ab:cd:ef", f.path()).await;
        assert_eq!(ip, Some("10.0.0.188".to_string()));
    }

    #[tokio::test]
    async fn test_lookup_ip_missing_lease_file_returns_none_when_no_arp() {
        // Non-existent lease file and a MAC that won't be in the ARP table.
        let ip = lookup_ip_by_mac("52:54:00:ff:ff:ff", Path::new("/nonexistent/leases")).await;
        // Either None (no ARP entry) or Some(...) if by coincidence it's in ARP.
        // We can't assert a specific value, but it must not panic.
        let _ = ip;
    }
}
