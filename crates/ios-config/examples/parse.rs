//! Parse a Cisco IOS config file and print a structured summary.
//!
//! Usage:
//!   cargo run --example parse -- path/to/running-config.txt
//!
//! Or pipe from clipboard:
//!   pbpaste | cargo run --example parse

use ios_config::parse;
use std::{env, fs, io::{self, Read}};

fn main() {
    let raw = match env::args().nth(1) {
        Some(path) => fs::read_to_string(&path)
            .unwrap_or_else(|e| { eprintln!("Cannot read {path}: {e}"); std::process::exit(1) }),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .unwrap_or_else(|e| { eprintln!("Cannot read stdin: {e}"); std::process::exit(1) });
            buf
        }
    };

    let cfg = match parse(&raw) {
        Ok(c) => c,
        Err(e) => { eprintln!("Parse error: {e}"); std::process::exit(1) }
    };

    // ── Header ────────────────────────────────────────────────────────────────
    println!("=== ios-config summary ===\n");
    if let Some(h) = &cfg.hostname      { println!("Hostname    : {h}"); }
    if let Some(d) = &cfg.domain_name   { println!("Domain      : {d}"); }

    // ── Interfaces ────────────────────────────────────────────────────────────
    println!("\n── Interfaces ({}) ──", cfg.interfaces.len());
    for iface in &cfg.interfaces {
        let state = if iface.shutdown { "shutdown" } else { "up" };
        let addrs: Vec<_> = iface.addresses.iter()
            .map(|a| a.prefix.to_string())
            .collect();
        let addr_str = if addrs.is_empty() {
            "no ip address".to_string()
        } else {
            addrs.join(", ")
        };
        let desc = iface.description.as_deref().unwrap_or("-");
        println!("  {:<32} [{state:^8}]  {addr_str}  \"{desc}\"",
            iface.name.original);
    }

    // ── Routing ───────────────────────────────────────────────────────────────
    if !cfg.routing.static_routes.is_empty() {
        println!("\n── Static routes ({}) ──", cfg.routing.static_routes.len());
        for r in &cfg.routing.static_routes {
            print!("  {}", r.prefix);
            match &r.next_hop {
                ios_config::NextHop::Ip(ip)          => print!(" via {ip}"),
                ios_config::NextHop::Interface(i)    => print!(" dev {i}"),
                ios_config::NextHop::IpAndInterface(ip, i) => print!(" via {ip} dev {i}"),
                ios_config::NextHop::Null0            => print!(" Null0"),
            }
            if let Some(name) = &r.name { print!(" ({name})"); }
            println!();
        }
    }

    for ospf in &cfg.routing.ospf {
        println!("\n── OSPF process {} ──", ospf.process_id);
        if let Some(rid) = &ospf.router_id { println!("  router-id {rid}"); }
        let net_count: usize = ospf.areas.iter().map(|a| a.networks.len()).sum();
        println!("  networks  : {net_count} across {} area(s)", ospf.areas.len());
        println!("  passive   : {}", ospf.passive_interfaces.join(", "));
    }

    if let Some(bgp) = &cfg.routing.bgp {
        println!("\n── BGP AS{} ──", bgp.asn);
        println!("  neighbors : {}", bgp.neighbors.len());
        println!("  peer-groups: {}", bgp.peer_groups.len());
        println!("  address-families: {}", bgp.address_families.len());
    }

    // ── Security ──────────────────────────────────────────────────────────────
    if !cfg.acls.is_empty() {
        println!("\n── ACLs ({}) ──", cfg.acls.len());
        for acl in &cfg.acls {
            println!("  {:?} ({} entries)", acl.name, acl.entries.len());
        }
    }

    if !cfg.nat.is_empty() {
        println!("\n── NAT ({} rules) ──", cfg.nat.len());
    }

    // ── Management ────────────────────────────────────────────────────────────
    if !cfg.ntp.is_empty() {
        let servers: Vec<_> = cfg.ntp.iter().map(|n| {
            if n.prefer { format!("{} (prefer)", n.address) } else { n.address.to_string() }
        }).collect();
        println!("\n── NTP: {} ──", servers.join(", "));
    }

    if let Some(snmp) = &cfg.snmp {
        println!("\n── SNMP ──");
        if let Some(loc) = &snmp.location { println!("  location: {loc}"); }
        println!("  communities: {}", snmp.communities.len());
    }

    // ── Unknowns ─────────────────────────────────────────────────────────────
    if !cfg.unknown_blocks.is_empty() {
        println!("\n── Unrecognized commands ({}) ──", cfg.unknown_blocks.len());
        for u in cfg.unknown_blocks.iter().take(10) {
            println!("  [{}] {}", u.context, u.raw.trim());
        }
        if cfg.unknown_blocks.len() > 10 {
            println!("  … and {} more", cfg.unknown_blocks.len() - 10);
        }
    }

    println!("\nDone.");
}
