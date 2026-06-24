# ios-config

A Rust library for parsing Cisco IOS `show running-config` output into a typed,
structured representation.

```toml
[dependencies]
ios-config = "0.1"
```

## Usage

```rust
use ios_config::parse;

let raw = std::fs::read_to_string("router.txt").unwrap();
let config = parse(&raw).unwrap();

println!("Hostname: {}", config.hostname.unwrap_or_default());
println!("Interfaces: {}", config.interfaces.len());
println!("OSPF processes: {}", config.routing.ospf.len());
println!("BGP ASN: {:?}", config.routing.bgp.map(|b| b.asn));
println!("ACLs: {}", config.acls.len());
println!("Unknown commands: {}", config.unknown_blocks.len());
```

## What gets parsed

| Section | Coverage |
|---------|----------|
| **Interfaces** | GigabitEthernet, FastEthernet, TenGigabitEthernet, Loopback, Vlan, Tunnel, Serial, Management — with abbreviated names (`Gi`, `Fa`, `Te`, `Lo`, …) |
| **L2** | `switchport access/trunk`, native/allowed VLANs, voice vlan, storm-control, per-interface STP (portfast, bpduguard, bpdufilter, guard root) |
| **IP** | Primary and secondary addresses, CIDR and dotted-mask formats, helper-address, HSRP (group, VIP, priority, preempt, timers, track) |
| **OSPF** | Multi-process, router-id, network statements, passive-interface, area types (stub/nssa), redistribution, default-originate, authentication (simple/MD5), per-interface cost/priority/timers/network-type |
| **BGP** | ASN, router-id, neighbors, peer-groups, address-families (ipv4/ipv6/vpnv4), route-maps, prefix-lists, community, next-hop-self, aggregate-address |
| **EIGRP** | ASN, network statements, passive-interface, redistribution |
| **Static routes** | Prefix, next-hop (IP / interface / both / Null0), AD, tag, name, permanent |
| **ACL** | Standard and extended, named and numbered, all match fields (protocol, src/dst, ports, established, log), remarks |
| **NAT** | Dynamic (pool), PAT/overload, static (including port-static), pool declarations |
| **VLANs** | `vlan <id>` blocks with name and state |
| **Management** | NTP (prefer, key, source), DNS name-servers, SNMP (communities, location, contact, traps), AAA (new-model, authentication/authorization lists), local users (privilege, password type), line vty (exec-timeout, transport-input), SSH (version, timeout, retries), banner motd |
| **Global STP** | Mode (rapid-pvst/pvst/mst), loopguard, portfast/bpduguard defaults, per-VLAN priority |
| **Unknown** | Unrecognized commands are preserved in `NetworkConfig::unknown_blocks` — nothing is silently dropped |

## Design

The parser runs in two passes:

1. **Structural pass** (`tree.rs`) — builds an indent-aware tree from raw text.
   No IOS semantics at this stage; purely whitespace-based hierarchy.
2. **Semantic pass** (`semantic.rs`) — walks the tree and fills the IR
   (`NetworkConfig`). Each recognized command maps to a typed struct field.
   Unrecognized commands go into `unknown_blocks`.

The intermediate representation lives in the companion crate `ios-config-core`
and is `serde`-serializable, so you can trivially dump parsed configs to JSON:

```rust
let config = ios_config::parse(&raw).unwrap();
let json = serde_json::to_string_pretty(&config).unwrap();
```

## Confidence field

Every `Interface` carries a `Confidence` field indicating parser certainty:

| Variant | Meaning |
|---------|---------|
| `Exact` | Fully recognized, lossless representation |
| `Approximate { note }` | Recognized with caveats (e.g. HSRP→VRRP semantic difference) |
| `Manual { reason }` | No structured representation possible |
| `Unknown { raw }` | Parser did not recognize the command |

## Workspace layout

```
ios-config/
├── crates/
│   ├── ios-config-core/   # IR types (NetworkConfig, Interface, BgpConfig, …)
│   └── ios-config/        # Parser — depends only on ios-config-core
└── Cargo.toml
```

## License

MIT
