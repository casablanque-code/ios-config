//! Intermediate representation (IR) for Cisco IOS configurations.
//!
//! All types are [`serde`]-serializable, so you can dump a parsed config
//! directly to JSON, TOML, or any other format:
//!
//! ```rust,ignore
//! let config = ios_config::parse(&raw).unwrap();
//! let json = serde_json::to_string_pretty(&config).unwrap();
//! ```

// Self-evident struct fields (e.g. `asn: u32`, `name: String`, `prefix: IpNet`)
// are exempt from the missing_docs lint — their types and names are documentation enough.
#![allow(missing_docs)]

use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// Parser confidence level for a recognized element.
///
/// Every [`Interface`] carries a `Confidence` value so callers can
/// distinguish between a cleanly parsed command and one that required
/// a best-effort interpretation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Confidence {
    /// The command was fully recognized and losslessly represented.
    Exact,
    /// An equivalent exists but with semantic differences; see `note`.
    Approximate { note: String },
    /// No structured representation is possible; see `reason`.
    Manual { reason: String },
    /// The parser did not recognize the command; raw text is preserved.
    Unknown { raw: String },
}

// ---------------------------------------------------------------------------
// NetworkConfig
// ---------------------------------------------------------------------------

/// Root object returned by the `ios_config::parse` function.
///
/// Every top-level section of a `show running-config` output maps to a
/// field here. Unrecognized commands are never silently dropped — they
/// are collected in [`unknown_blocks`][NetworkConfig::unknown_blocks].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Value of the `hostname` command.
    pub hostname: Option<String>,
    /// Value of the `ip domain-name` command.
    pub domain_name: Option<String>,
    /// All `interface` blocks in config order.
    pub interfaces: Vec<Interface>,
    /// All `vlan <id>` database blocks.
    pub vlans: Vec<Vlan>,
    /// Routing table: static routes, OSPF, BGP, EIGRP.
    pub routing: RoutingConfig,
    /// Named and numbered access control lists.
    pub acls: Vec<Acl>,
    /// NAT rules (`ip nat inside source …`).
    pub nat: Vec<NatRule>,
    /// NTP servers (`ntp server …`).
    pub ntp: Vec<NtpServer>,
    /// DNS name servers (`ip name-server …`), in config order.
    pub dns: Vec<IpAddr>,
    /// SNMP configuration (`snmp-server …`).
    pub snmp: Option<SnmpConfig>,
    /// AAA configuration (`aaa new-model`, authentication/authorization lists).
    pub aaa: Option<AaaConfig>,
    /// Global spanning-tree settings (`spanning-tree mode …`).
    pub stp: Option<GlobalStp>,
    /// Logging configuration (`logging buffered …`, `logging host …`).
    pub logging: Option<LoggingConfig>,
    /// Local user accounts (`username … privilege … secret …`).
    pub users: Vec<LocalUser>,
    /// `line vty` configuration (exec-timeout, transport-input).
    pub line_vty: Option<LineVty>,
    /// `ip ssh` configuration.
    pub ssh: Option<SshConfig>,
    /// `banner motd` text (delimiters stripped).
    pub banner: Option<String>,
    /// Commands the parser did not recognize, grouped by their config block.
    /// Nothing is silently dropped — use this to audit parse coverage.
    pub unknown_blocks: Vec<UnknownBlock>,
    /// Platform-specific commands that have no vendor-neutral equivalent.
    pub platform_specific: Vec<UnknownBlock>,
}

// ---------------------------------------------------------------------------
// Global STP
// ---------------------------------------------------------------------------

/// Global spanning-tree configuration (`spanning-tree …` at global level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStp {
    /// STP mode (`rapid-pvst`, `pvst`, `mst`, `rstp`).
    pub mode: StpMode,
    /// `spanning-tree loopguard default`
    pub loopguard: bool,
    /// `spanning-tree portfast default`
    pub portfast_default: bool,
    /// `spanning-tree portfast bpduguard default`
    pub bpduguard_default: bool,
    /// Per-VLAN priority overrides (`spanning-tree vlan <id> priority <n>`).
    pub vlan_priorities: Vec<StpVlanPriority>,
}

/// STP operational mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StpMode {
    RapidPvst,
    Pvst,
    Mst,
    Rstp,
}

/// Per-VLAN STP priority override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StpVlanPriority {
    /// VLAN IDs this priority applies to.
    pub vlans: Vec<u16>,
    /// STP bridge priority (must be a multiple of 4096).
    pub priority: u32,
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Logging subsystem configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// `logging buffered <size>` — internal buffer size in bytes.
    pub buffered_size: Option<u32>,
    /// `logging console <level>` — severity level keyword (e.g. `"warnings"`).
    pub console_level: Option<String>,
    /// Remote syslog hosts (`logging host <ip>`).
    pub hosts: Vec<std::net::IpAddr>,
}

// ---------------------------------------------------------------------------
// Local users
// ---------------------------------------------------------------------------

/// A local user account defined by `username … privilege … secret/password …`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    pub name: String,
    /// Privilege level (0–15).
    pub privilege: u8,
    /// Hash algorithm used to store the password.
    pub password_type: PasswordType,
    /// Password string exactly as it appears in the config
    /// (plaintext, Type-7 encoded, or hash).
    pub password_hash: String,
}

/// Password storage type as encoded in the config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PasswordType {
    /// `password 0 …` or bare `password …` — cleartext.
    Plaintext,
    /// `password 7 …` — Cisco Type-7 (XOR-based, reversible).
    Type7,
    /// `secret 5 …` — MD5 hash (`$1$…`).
    Md5,
    /// `secret 9 …` — scrypt hash (`$9$…`).
    Scrypt,
}

// ---------------------------------------------------------------------------
// Line VTY
// ---------------------------------------------------------------------------

/// `line vty 0 4` (or `0 15`) configuration block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineVty {
    /// `exec-timeout <min>` portion.
    pub exec_timeout_min: u32,
    /// `exec-timeout <min> <sec>` seconds portion.
    pub exec_timeout_sec: u32,
    /// `transport input` protocols (e.g. `["ssh"]`, `["ssh", "telnet"]`).
    pub transport_input: Vec<String>,
    /// `logging synchronous`
    pub logging_synchronous: bool,
}

// ---------------------------------------------------------------------------
// SSH
// ---------------------------------------------------------------------------

/// `ip ssh` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    /// `ip ssh version <n>` — 1 or 2.
    pub version: u8,
    /// `ip ssh time-out <seconds>`
    pub timeout: Option<u32>,
    /// `ip ssh authentication-retries <n>`
    pub retries: Option<u8>,
}

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

/// A single `interface …` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    /// Parsed and normalized interface name.
    pub name: InterfaceName,
    /// `description` line text.
    pub description: Option<String>,
    /// IP addresses (primary first, then secondary).
    pub addresses: Vec<IpAddress>,
    /// `shutdown` is present (true) or absent (false).
    pub shutdown: bool,
    /// `mtu <bytes>`
    pub mtu: Option<u32>,
    /// `speed <mbps>` or `speed auto`.
    pub speed: Option<InterfaceSpeed>,
    /// `duplex full/half/auto`
    pub duplex: Option<Duplex>,
    /// Layer-2 switchport configuration, if present.
    pub l2: Option<L2Config>,
    /// `ip helper-address` entries.
    pub helper_addresses: Vec<IpAddr>,
    /// Inbound ACL name (`ip access-group … in`).
    pub acl_in: Option<String>,
    /// Outbound ACL name (`ip access-group … out`).
    pub acl_out: Option<String>,
    /// `ip nat inside` / `ip nat outside`.
    pub nat_direction: Option<NatDirection>,
    /// HSRP groups configured on this interface.
    pub hsrp: Vec<HsrpGroup>,
    /// Per-interface OSPF settings (`ip ospf …`).
    pub ospf: Option<InterfaceOspf>,
    /// `switchport voice vlan <id>`
    pub voice_vlan: Option<u16>,
    /// `storm-control` thresholds.
    pub storm_control: Option<StormControl>,
    /// Per-interface STP settings.
    pub stp: InterfaceStp,
    /// Parser confidence for this interface block.
    pub confidence: Confidence,
}

/// `storm-control` broadcast/multicast/unicast levels (percentage).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StormControl {
    pub broadcast_level: Option<f32>,
    pub multicast_level: Option<f32>,
    pub unicast_level: Option<f32>,
}

/// Per-interface spanning-tree settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterfaceStp {
    /// `spanning-tree portfast`
    pub portfast: bool,
    /// `spanning-tree bpduguard enable`
    pub bpduguard: bool,
    /// `spanning-tree bpdufilter enable`
    pub bpdufilter: bool,
    /// `spanning-tree guard root`
    pub guard_root: bool,
}

/// Parsed and normalized interface name.
///
/// Abbreviated names (`Gi`, `Fa`, `Te`, `Lo`, …) are expanded to their
/// canonical long form. The original string is preserved in [`original`][InterfaceName::original].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceName {
    /// Interface type.
    pub kind: InterfaceKind,
    /// Slot/port identifier as a string: `"0/0"`, `"0/0/0"`, `"1"`, etc.
    pub id: String,
    /// Verbatim name from the config, before any normalization.
    pub original: String,
}

/// Interface type discriminant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InterfaceKind {
    GigabitEthernet,
    FastEthernet,
    TenGigabitEthernet,
    Loopback,
    /// Layer-3 VLAN interface (`interface Vlan<id>`).
    Vlan,
    Tunnel,
    Serial,
    BundleEther,
    Management,
    /// Any interface type not matched by the parser.
    Unknown(String),
}

/// An IPv4 address/prefix assigned to an interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddress {
    /// Address and prefix length in CIDR notation.
    pub prefix: IpNet,
    /// True if this is a `secondary` address.
    pub secondary: bool,
}

/// Interface speed configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterfaceSpeed {
    /// Explicit speed in Mbps (`speed 100`, `speed 1000`, …).
    Mbps(u32),
    /// `speed auto`
    Auto,
}

/// Interface duplex setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Duplex {
    Full,
    Half,
    Auto,
}

/// Layer-2 switchport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Config {
    /// `switchport mode access` or `switchport mode trunk`.
    pub mode: L2Mode,
    /// `switchport access vlan <id>`
    pub access_vlan: Option<u16>,
    /// `switchport trunk allowed vlan …` — expanded VLAN list.
    pub trunk_allowed: Option<Vec<u16>>,
    /// `switchport trunk native vlan <id>`
    pub trunk_native: Option<u16>,
}

/// Switchport operating mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum L2Mode {
    Access,
    Trunk,
}

/// `ip nat inside` / `ip nat outside` marker on an interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NatDirection {
    Inside,
    Outside,
}

// ---------------------------------------------------------------------------
// HSRP
// ---------------------------------------------------------------------------

/// An HSRP group configured on an interface.
///
/// Note: when converting to platforms that use VRRP (e.g. Huawei VRP),
/// the parser sets [`Interface::confidence`] to [`Confidence::Approximate`]
/// because HSRP and VRRP have different timer semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsrpGroup {
    pub group_id: u16,
    pub virtual_ip: IpAddr,
    /// `standby <id> priority <n>` — default 100.
    pub priority: Option<u16>,
    /// `standby <id> preempt`
    pub preempt: bool,
    /// `standby <id> preempt delay minimum <sec>`
    pub preempt_delay: Option<u32>,
    pub timers: Option<HsrpTimers>,
    pub track: Vec<HsrpTrack>,
}

/// HSRP hello/hold timers in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsrpTimers {
    pub hello_ms: u32,
    pub hold_ms: u32,
}

/// `standby <id> track <object> decrement <n>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsrpTrack {
    pub object: u32,
    pub decrement: u16,
}

// ---------------------------------------------------------------------------
// OSPF (per-interface)
// ---------------------------------------------------------------------------

/// Per-interface OSPF parameters (`ip ospf …`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceOspf {
    pub process_id: u32,
    pub area: OspfArea,
    /// `ip ospf cost <n>`
    pub cost: Option<u32>,
    /// `ip ospf priority <n>`
    pub priority: Option<u8>,
    pub timers: Option<OspfIfTimers>,
    pub auth: Option<OspfAuth>,
    /// `ip ospf passive-interface`
    pub passive: bool,
    /// `ip ospf network <type>`
    pub network_type: Option<OspfNetworkType>,
}

/// OSPF interface hello/dead timers (seconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfIfTimers {
    pub hello_interval: u32,
    pub dead_interval: u32,
}

// ---------------------------------------------------------------------------
// VLAN
// ---------------------------------------------------------------------------

/// A `vlan <id>` database entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vlan {
    pub id: u16,
    /// `name <string>`
    pub name: Option<String>,
    /// False if `state suspend` is set.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

/// All routing configuration parsed from the config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// `ip route …` entries.
    pub static_routes: Vec<StaticRoute>,
    /// `router ospf <pid>` blocks (multiple processes supported).
    pub ospf: Vec<OspfProcess>,
    /// `router bgp <asn>` block (at most one per device).
    pub bgp: Option<BgpConfig>,
    /// `router eigrp <asn>` blocks.
    pub eigrp: Vec<EigrpProcess>,
}

/// A single `ip route <prefix> …` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticRoute {
    pub prefix: IpNet,
    pub next_hop: NextHop,
    /// Administrative distance (default 1 if absent).
    pub distance: Option<u8>,
    pub tag: Option<u32>,
    /// `name <string>` label.
    pub name: Option<String>,
    /// `permanent` flag — route survives interface going down.
    pub permanent: bool,
}

/// Next-hop specification for a static route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NextHop {
    /// `ip route … <next-hop-ip>`
    Ip(IpAddr),
    /// `ip route … <exit-interface>`
    Interface(String),
    /// `ip route … <exit-interface> <next-hop-ip>`
    IpAndInterface(IpAddr, String),
    /// `ip route … Null0`
    Null0,
}

/// A `router ospf <pid>` process block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfProcess {
    pub process_id: u32,
    pub router_id: Option<IpAddr>,
    /// Area configurations including `network` statements.
    pub areas: Vec<OspfAreaConfig>,
    /// Interfaces excluded from OSPF hellos (`passive-interface`).
    pub passive_interfaces: Vec<String>,
    /// `default-information originate` settings.
    pub default_originate: Option<OspfDefaultOriginate>,
    /// Redistributed protocol sources.
    pub redistribute: Vec<OspfRedistribute>,
    /// `max-metric router-lsa` (stub router advertisement).
    pub max_metric: bool,
    /// Process-level authentication.
    pub auth: Option<OspfAuth>,
    /// `log-adjacency-changes`
    pub log_adjacency: bool,
}

/// Per-area OSPF configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfAreaConfig {
    pub area: OspfArea,
    /// `network <address> <wildcard> area <id>` entries.
    pub networks: Vec<OspfNetwork>,
    pub area_type: OspfAreaType,
    pub auth: Option<OspfAuth>,
}

/// OSPF area identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OspfArea {
    /// Area 0 (backbone), regardless of how it was written in config.
    Backbone,
    /// `area <n>` where n > 0.
    Normal(u32),
    /// `area 0.0.0.1` dotted-decimal format.
    IpFormat(IpAddr),
}

/// A `network <addr> <wildcard> area <id>` statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfNetwork {
    /// The network prefix. If the config used a wildcard mask,
    /// it is converted to prefix length.
    pub prefix: IpNet,
    /// True if the original config used a wildcard mask (not prefix length).
    pub wildcard: bool,
}

/// OSPF area type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OspfAreaType {
    Normal,
    Stub,
    /// `area <id> stub no-summary`
    StubNoSummary,
    Nssa,
    /// `area <id> nssa no-summary`
    NssaNoSummary,
}

/// OSPF authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OspfAuth {
    /// `ip ospf authentication-key <key>` (plaintext).
    Simple(String),
    /// `ip ospf message-digest-key <id> md5 <key>`.
    Md5 { key_id: u8, key: String },
}

/// OSPF network type for an interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OspfNetworkType {
    Broadcast,
    PointToPoint,
    PointToMultipoint,
    NonBroadcast,
}

/// `default-information originate` parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfDefaultOriginate {
    /// `always` keyword — advertise even without a default route in the table.
    pub always: bool,
    pub metric: Option<u32>,
    /// Metric type: 1 (E1) or 2 (E2, default).
    pub metric_type: Option<u8>,
}

/// A `redistribute <source> …` statement inside a routing process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OspfRedistribute {
    pub source: RedistributeSource,
    pub metric: Option<u32>,
    pub metric_type: Option<u8>,
    /// `subnets` keyword (required for redistributing into OSPF).
    pub subnets: bool,
    pub tag: Option<u32>,
    pub route_map: Option<String>,
}

/// Source protocol for redistribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedistributeSource {
    Connected,
    Static,
    Bgp(u32),
    Eigrp(u32),
    Rip,
}

// ---------------------------------------------------------------------------
// BGP
// ---------------------------------------------------------------------------

/// A `router bgp <asn>` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpConfig {
    pub asn: u32,
    pub router_id: Option<IpAddr>,
    /// All `neighbor` entries defined at the global BGP level.
    pub neighbors: Vec<BgpNeighbor>,
    pub peer_groups: Vec<BgpPeerGroup>,
    /// `network` statements in the global context (outside any address-family).
    pub networks: Vec<IpNet>,
    /// `address-family` blocks.
    pub address_families: Vec<BgpAddressFamily>,
    pub redistribute: Vec<OspfRedistribute>,
    /// `bgp log-neighbor-changes`
    pub log_neighbor_changes: bool,
    /// `bgp bestpath …` policy string, if present.
    pub bestpath: Option<String>,
}

/// A BGP neighbor entry (`neighbor <addr|peer-group> …`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpNeighbor {
    pub address: BgpNeighborAddr,
    pub remote_as: u32,
    pub description: Option<String>,
    /// `update-source <interface>`
    pub update_source: Option<String>,
    /// `next-hop-self`
    pub next_hop_self: bool,
    pub password: Option<String>,
    pub shutdown: bool,
    /// Peer-group this neighbor inherits from.
    pub peer_group: Option<String>,
    pub route_map_in: Option<String>,
    pub route_map_out: Option<String>,
    pub prefix_list_in: Option<String>,
    pub prefix_list_out: Option<String>,
    pub soft_reconfiguration: bool,
    pub send_community: bool,
    pub remove_private_as: bool,
    pub default_originate: bool,
    pub activate: bool,
}

/// BGP neighbor address — either an IP or a peer-group name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BgpNeighborAddr {
    Ip(IpAddr),
    PeerGroup(String),
}

impl std::fmt::Display for BgpNeighborAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BgpNeighborAddr::Ip(ip) => write!(f, "{}", ip),
            BgpNeighborAddr::PeerGroup(name) => write!(f, "{}", name),
        }
    }
}

/// A BGP peer-group definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpPeerGroup {
    pub name: String,
    pub remote_as: Option<u32>,
    pub update_source: Option<String>,
    pub next_hop_self: bool,
    pub route_map_in: Option<String>,
    pub route_map_out: Option<String>,
    pub send_community: bool,
}

/// A BGP `address-family <afi> <safi>` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpAddressFamily {
    pub afi: BgpAfi,
    pub safi: BgpSafi,
    /// `network` statements inside this address-family.
    pub networks: Vec<IpNet>,
    pub redistribute: Vec<OspfRedistribute>,
    /// Neighbors activated with `neighbor <addr> activate`.
    pub activated_neighbors: Vec<BgpNeighborAddr>,
    /// Neighbors explicitly deactivated with `no neighbor <addr> activate`.
    pub deactivated_neighbors: Vec<BgpNeighborAddr>,
    /// Per-neighbor settings overridden inside this address-family.
    pub neighbor_settings: Vec<BgpNeighbor>,
    pub default_information: bool,
    /// `aggregate-address` entries.
    pub aggregate_addresses: Vec<BgpAggregate>,
}

/// BGP address family indicator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BgpAfi {
    Ipv4,
    Ipv6,
    Vpnv4,
    L2vpn,
}

/// BGP subsequent address family indicator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BgpSafi {
    Unicast,
    Multicast,
    Labeled,
    Evpn,
}

/// A `aggregate-address <prefix> …` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpAggregate {
    pub prefix: IpNet,
    /// `summary-only` — suppress more-specific prefixes.
    pub summary_only: bool,
    /// `as-set` — include AS path information from contributing routes.
    pub as_set: bool,
}

/// A `router eigrp <asn>` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigrpProcess {
    pub asn: u32,
    pub networks: Vec<OspfNetwork>,
    pub passive_interfaces: Vec<String>,
    pub redistribute: Vec<OspfRedistribute>,
}

// ---------------------------------------------------------------------------
// ACL
// ---------------------------------------------------------------------------

/// An IP access control list (standard or extended, named or numbered).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acl {
    pub name: AclName,
    pub acl_type: AclType,
    pub entries: Vec<AclEntry>,
}

/// ACL identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AclName {
    /// `ip access-list standard|extended <name>`
    Named(String),
    /// `access-list <number> …`
    Numbered(u32),
}

/// ACL type controlling which fields are matched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AclType {
    /// Standard — matches source IP only.
    Standard,
    /// Extended — matches source, destination, protocol, and ports.
    Extended,
}

/// A single ACE (access control entry) line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclEntry {
    /// Explicit sequence number if present (`10 permit …`).
    pub sequence: Option<u32>,
    pub action: AclAction,
    pub protocol: Option<AclProtocol>,
    pub src: AclMatch,
    pub dst: Option<AclMatch>,
    pub src_port: Option<AclPort>,
    pub dst_port: Option<AclPort>,
    /// `established` keyword (TCP only).
    pub established: bool,
    /// `log` keyword.
    pub log: bool,
    /// `remark <text>` line.
    pub remark: Option<String>,
}

/// ACE action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AclAction {
    Permit,
    Deny,
}

/// Protocol field in an extended ACE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AclProtocol {
    Ip,
    Tcp,
    Udp,
    Icmp,
    Esp,
    Ahp,
    Number(u8),
}

/// Address match criterion in an ACE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AclMatch {
    /// `any`
    Any,
    /// `host <ip>`
    Host(IpAddr),
    /// `<addr> <wildcard>` — stored as-is for fidelity.
    Network { addr: IpAddr, wildcard: IpAddr },
    /// CIDR prefix (used when the config already specifies prefix length).
    Prefix(IpNet),
}

/// Port match operator in an extended ACE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AclPort {
    Eq(u16),
    Ne(u16),
    Lt(u16),
    Gt(u16),
    Range(u16, u16),
}

// ---------------------------------------------------------------------------
// NAT
// ---------------------------------------------------------------------------

/// A NAT rule (`ip nat inside source …`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatRule {
    pub rule_type: NatType,
    /// Source ACL name (for dynamic/PAT NAT).
    pub acl: Option<String>,
    /// Address pool (resolved even when declared after the `ip nat` statement).
    pub pool: Option<NatPool>,
    /// True when `interface <if> overload` is used instead of a pool.
    pub interface_overload: bool,
    /// Static NAT mapping entry, if this is a static rule.
    pub static_entry: Option<NatStaticEntry>,
}

/// NAT rule type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NatType {
    /// `ip nat inside source list <acl> pool <pool>`
    Dynamic,
    /// `ip nat inside source list <acl> interface <if> overload` (PAT).
    Overload,
    /// `ip nat inside source static <local> <global>`
    Static,
}

/// An `ip nat pool <name> …` definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatPool {
    pub name: String,
    pub start: IpAddr,
    pub end: IpAddr,
    /// Network prefix if `prefix-length` was specified.
    pub prefix: Option<IpNet>,
    pub overload: bool,
}

/// A static NAT mapping (`ip nat inside source static …`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatStaticEntry {
    pub local: IpAddr,
    pub global: IpAddr,
    /// Port for static port-NAT (`local_port` / `global_port` may differ).
    pub local_port: Option<u16>,
    pub global_port: Option<u16>,
    pub protocol: Option<AclProtocol>,
}

// ---------------------------------------------------------------------------
// NTP / DNS / SNMP / AAA
// ---------------------------------------------------------------------------

/// An `ntp server …` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtpServer {
    pub address: IpAddr,
    /// `prefer` keyword.
    pub prefer: bool,
    /// `key <id>` for NTP authentication.
    pub key: Option<u32>,
    /// `source <interface>` for NTP packets.
    pub source_interface: Option<String>,
}

/// `snmp-server` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnmpConfig {
    /// Community strings with access level and optional ACL.
    pub communities: Vec<SnmpCommunity>,
    /// `snmp-server location <text>`
    pub location: Option<String>,
    /// `snmp-server contact <text>`
    pub contact: Option<String>,
    /// Enabled trap types (`snmp-server enable traps …`).
    pub traps: Vec<String>,
}

/// An `snmp-server community <name> RO|RW` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnmpCommunity {
    pub name: String,
    pub access: SnmpAccess,
    /// Optional ACL restricting which hosts can use this community.
    pub acl: Option<String>,
}

/// SNMP community access level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnmpAccess {
    Ro,
    Rw,
}

/// `aaa new-model` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaaConfig {
    pub new_model: bool,
    /// `aaa authentication …` method lists.
    pub authentication: Vec<AaaMethod>,
    /// `aaa authorization …` method lists.
    pub authorization: Vec<AaaMethod>,
}

/// An AAA method list entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaaMethod {
    /// List name (`default` or custom).
    pub list_name: String,
    /// Ordered list of authentication/authorization methods.
    pub methods: Vec<String>,
}

// ---------------------------------------------------------------------------
// UnknownBlock
// ---------------------------------------------------------------------------

/// A command that the parser did not recognize.
///
/// Unrecognized commands are never silently discarded. Inspect
/// [`NetworkConfig::unknown_blocks`] to audit what the parser missed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownBlock {
    /// Source line number in the original config (0 if unavailable).
    pub line: usize,
    /// Config block context, e.g. `"global"`, `"interface GigabitEthernet0/0"`,
    /// `"router ospf 1"`.
    pub context: String,
    /// Verbatim command text.
    pub raw: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub type ProcessId = u32;

impl Default for Interface {
    fn default() -> Self {
        Interface {
            name: InterfaceName {
                kind: InterfaceKind::Unknown(String::new()),
                id: String::new(),
                original: String::new(),
            },
            description: None,
            addresses: vec![],
            shutdown: false,
            mtu: None,
            speed: None,
            duplex: None,
            l2: None,
            helper_addresses: vec![],
            acl_in: None,
            acl_out: None,
            nat_direction: None,
            hsrp: vec![],
            ospf: None,
            voice_vlan: None,
            storm_control: None,
            stp: InterfaceStp::default(),
            confidence: Confidence::Exact,
        }
    }
}

impl InterfaceName {
    /// Parse a raw interface name string, expanding Cisco abbreviations.
    ///
    /// ```
    /// use ios_config_core::ir::{InterfaceName, InterfaceKind};
    /// let name = InterfaceName::parse("Gi0/1");
    /// assert_eq!(name.kind, InterfaceKind::GigabitEthernet);
    /// assert_eq!(name.id, "0/1");
    /// assert_eq!(name.original, "Gi0/1");
    /// ```
    pub fn parse(raw: &str) -> Self {
        let original = raw.to_string();
        let expanded = expand_interface_name(raw);
        let (kind_str, id) = split_interface_name(&expanded);

        let kind = match kind_str.to_lowercase().as_str() {
            "gigabitethernet" => InterfaceKind::GigabitEthernet,
            "fastethernet" => InterfaceKind::FastEthernet,
            "tengigabitethernet" | "tengige" => InterfaceKind::TenGigabitEthernet,
            "loopback" => InterfaceKind::Loopback,
            "vlan" => InterfaceKind::Vlan,
            "tunnel" => InterfaceKind::Tunnel,
            "serial" => InterfaceKind::Serial,
            "bundle-ether" | "bundleether" => InterfaceKind::BundleEther,
            "management" => InterfaceKind::Management,
            other => InterfaceKind::Unknown(other.to_string()),
        };

        InterfaceName { kind, id, original }
    }
}

fn expand_interface_name(raw: &str) -> String {
    let prefixes = [
        ("Gi", "GigabitEthernet"),
        ("Fa", "FastEthernet"),
        ("Te", "TenGigabitEthernet"),
        ("Lo", "Loopback"),
        ("Tu", "Tunnel"),
        ("Se", "Serial"),
        ("Vl", "Vlan"),
        ("Mg", "Management"),
    ];
    for (short, full) in &prefixes {
        if raw.starts_with(short) && !raw.to_lowercase().starts_with(&full.to_lowercase()) {
            return format!("{}{}", full, &raw[short.len()..]);
        }
    }
    raw.to_string()
}

fn split_interface_name(name: &str) -> (&str, String) {
    let pos = name.find(|c: char| c.is_ascii_digit());
    match pos {
        Some(i) => (&name[..i], name[i..].to_string()),
        None => (name, String::new()),
    }
}

impl OspfArea {
    /// Parse an area identifier string (`"0"`, `"1"`, `"0.0.0.1"`, …).
    pub fn parse(s: &str) -> Self {
        if let Ok(n) = s.parse::<u32>() {
            if n == 0 { OspfArea::Backbone } else { OspfArea::Normal(n) }
        } else if let Ok(ip) = s.parse::<IpAddr>() {
            if ip == "0.0.0.0".parse::<IpAddr>().unwrap() {
                OspfArea::Backbone
            } else {
                OspfArea::IpFormat(ip)
            }
        } else {
            OspfArea::Normal(0)
        }
    }
}
