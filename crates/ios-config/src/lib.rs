//! # ios-config
//!
//! A Cisco IOS `show running-config` parser that produces a typed intermediate
//! representation (IR) covering interfaces, routing (OSPF, BGP, static),
//! ACLs, NAT, VLANs, SNMP, AAA, STP, and more.
#![deny(missing_docs)]
//!
//! ## Quick start
//!
//! ```rust
//! use ios_config::parse;
//!
//! let raw = r#"
//! hostname CORE-RTR-01
//! !
//! interface GigabitEthernet0/0
//!  description WAN
//!  ip address 203.0.113.1 255.255.255.252
//!  no shutdown
//! !
//! router ospf 1
//!  router-id 1.1.1.1
//!  network 203.0.113.0 0.0.0.3 area 0
//! "#;
//!
//! let config = parse(raw).unwrap();
//! assert_eq!(config.hostname.as_deref(), Some("CORE-RTR-01"));
//! assert_eq!(config.interfaces.len(), 1);
//! assert_eq!(config.routing.ospf.len(), 1);
//! ```
//!
//! ## What gets parsed
//!
//! | Section | Types |
//! |---------|-------|
//! | Interfaces | GigabitEthernet, FastEthernet, TenGigabitEthernet, Loopback, Vlan, Tunnel, Serial |
//! | L2 | switchport access/trunk, voice vlan, storm-control, STP per-interface |
//! | Routing | static routes, OSPF (multi-process), BGP (neighbors, peer-groups, address-families), EIGRP |
//! | ACL | standard and extended, named and numbered |
//! | NAT | dynamic, PAT/overload, static |
//! | Management | NTP, DNS, SNMP, AAA, SSH, line vty, local users, banner |
//! | Global STP | mode, loopguard, portfast/bpduguard defaults, per-VLAN priority |
//! | Unknown | unrecognized commands preserved in `unknown_blocks` |

pub use ios_config_core::ir::{
    NetworkConfig,
    Interface, InterfaceName, InterfaceKind, InterfaceSpeed, Duplex,
    IpAddress, L2Config, L2Mode, NatDirection,
    HsrpGroup, HsrpTimers, HsrpTrack,
    InterfaceOspf, OspfIfTimers, OspfNetworkType,
    InterfaceStp, StormControl,
    Vlan,
    RoutingConfig, StaticRoute, NextHop,
    OspfProcess, OspfAreaConfig, OspfArea, OspfNetwork, OspfAreaType,
    OspfAuth, OspfDefaultOriginate, OspfRedistribute, RedistributeSource,
    BgpConfig, BgpNeighbor, BgpNeighborAddr, BgpPeerGroup,
    BgpAddressFamily, BgpAfi, BgpSafi, BgpAggregate,
    EigrpProcess,
    Acl, AclName, AclType, AclEntry, AclAction, AclProtocol, AclMatch, AclPort,
    NatRule, NatType, NatPool, NatStaticEntry,
    NtpServer, SnmpConfig, SnmpCommunity, SnmpAccess,
    AaaConfig, AaaMethod,
    GlobalStp, StpMode, StpVlanPriority,
    LoggingConfig, LocalUser, PasswordType,
    LineVty, SshConfig,
    Confidence,
    UnknownBlock,
};

mod tree;
mod semantic;

#[cfg(test)]
mod tests;

use tree::parse_raw_tree;
use semantic::SemanticParser;

/// Parse errors returned by [`parse`].
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Input was empty or contained no recognizable IOS commands.
    EmptyInput,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "ios-config: input is empty"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a Cisco IOS `show running-config` string into a [`NetworkConfig`].
///
/// The parser is lenient: unrecognized commands are preserved in
/// [`NetworkConfig::unknown_blocks`] rather than causing an error.
///
/// # Errors
///
/// Returns [`ParseError::EmptyInput`] if the input contains no non-whitespace,
/// non-comment lines.
pub fn parse(input: &str) -> Result<NetworkConfig, ParseError> {
    let has_content = input.lines()
        .any(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('!')
        });

    if !has_content {
        return Err(ParseError::EmptyInput);
    }

    let tree = parse_raw_tree(input);
    let config = SemanticParser.analyze(&tree);
    Ok(config)
}

#[cfg(test)]
mod doctest_examples {
    use super::*;

    const SAMPLE: &str = r#"
hostname EDGE-RTR-01
ip domain-name corp.local
!
interface GigabitEthernet0/0
 description WAN Uplink
 ip address 203.0.113.2 255.255.255.252
 ip nat outside
 no shutdown
!
interface GigabitEthernet0/1
 description LAN
 ip address 10.0.0.1 255.255.255.0
 ip nat inside
 ip helper-address 10.0.0.254
 no shutdown
!
interface Vlan10
 description Management
 ip address 10.10.10.1 255.255.255.0
 no shutdown
!
router ospf 1
 router-id 1.1.1.1
 network 10.0.0.0 0.0.0.255 area 0
 network 10.10.10.0 0.0.0.255 area 0
 passive-interface GigabitEthernet0/1
!
ip access-list extended ACL-WAN-IN
 permit tcp any any established
 deny   ip any any log
!
ip nat inside source list ACL-NAT interface GigabitEthernet0/0 overload
!
ntp server 216.239.35.0 prefer
ip name-server 8.8.8.8
ip name-server 8.8.4.4
!
snmp-server community public RO
snmp-server location DC-1 Rack-12
"#;

    #[test]
    fn test_parse_hostname() {
        let cfg = parse(SAMPLE).unwrap();
        assert_eq!(cfg.hostname.as_deref(), Some("EDGE-RTR-01"));
        assert_eq!(cfg.domain_name.as_deref(), Some("corp.local"));
    }

    #[test]
    fn test_parse_interfaces() {
        let cfg = parse(SAMPLE).unwrap();
        assert_eq!(cfg.interfaces.len(), 3);

        let wan = cfg.interfaces.iter().find(|i| i.name.original == "GigabitEthernet0/0").unwrap();
        assert_eq!(wan.description.as_deref(), Some("WAN Uplink"));
        assert!(!wan.shutdown);
        assert_eq!(wan.nat_direction, Some(NatDirection::Outside));
        assert_eq!(wan.addresses.len(), 1);
    }

    #[test]
    fn test_parse_ospf() {
        let cfg = parse(SAMPLE).unwrap();
        assert_eq!(cfg.routing.ospf.len(), 1);
        let ospf = &cfg.routing.ospf[0];
        assert_eq!(ospf.process_id, 1);
        assert_eq!(ospf.router_id, Some("1.1.1.1".parse().unwrap()));
        assert_eq!(ospf.passive_interfaces, vec!["GigabitEthernet0/1"]);
    }

    #[test]
    fn test_parse_acl() {
        let cfg = parse(SAMPLE).unwrap();
        assert_eq!(cfg.acls.len(), 1);
        let acl = &cfg.acls[0];
        assert!(matches!(acl.name, AclName::Named(ref n) if n == "ACL-WAN-IN"));
        assert_eq!(acl.acl_type, AclType::Extended);
        assert_eq!(acl.entries.len(), 2);
    }

    #[test]
    fn test_parse_ntp_dns_snmp() {
        let cfg = parse(SAMPLE).unwrap();
        assert_eq!(cfg.ntp.len(), 1);
        assert!(cfg.ntp[0].prefer);
        assert_eq!(cfg.dns.len(), 2);
        let snmp = cfg.snmp.as_ref().unwrap();
        assert_eq!(snmp.communities.len(), 1);
        assert_eq!(snmp.location.as_deref(), Some("DC-1 Rack-12"));
    }

    #[test]
    fn test_empty_input_error() {
        assert!(matches!(parse(""), Err(ParseError::EmptyInput)));
        assert!(matches!(parse("! just a comment\n!"), Err(ParseError::EmptyInput)));
    }

    #[test]
    fn test_interface_name_normalization() {
        let cfg = parse("interface Gi0/0\n ip address 192.168.1.1 255.255.255.0\n").unwrap();
        assert_eq!(cfg.interfaces[0].name.kind, InterfaceKind::GigabitEthernet);
        assert_eq!(cfg.interfaces[0].name.id, "0/0");
    }
}
