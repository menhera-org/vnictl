pub(crate) mod config;
pub(crate) mod cli;
pub(crate) mod db;

use clap::Parser;

use std::net::IpAddr;

fn main() {
    let args = cli::Cli::parse();

    let config: config::Config = toml::from_str(&std::fs::read_to_string(&args.config).unwrap()).unwrap();

    let db = db::Database::open(&config.db_path, config.start_vlan).unwrap();

    match args.subcmd {
        cli::Command::Enable { vni } => enable(&config, &db, vni),
        cli::Command::Disable { vni } => disable(&config, &db, vni),
        cli::Command::List => list(&config, &db),
        cli::Command::Status { vni } => status(&config, &db, vni),
    }
}

fn check_for_root() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("This program must be run as root");
        std::process::exit(1);
    }
}

fn reload_networkd() {
    let output = std::process::Command::new("networkctl")
        .arg("reload")
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("Failed to reload systemd-networkd: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
}

fn ovs_add_internal_port(br: &str, vni: u32, vlan: u16) {
    let port = &format!("vlink{}", vni);
    let output = std::process::Command::new("ovs-vsctl")
        .arg("add-port")
        .arg(br)
        .arg(port)
        .arg(&format!("tag={}", vlan))
        .arg("--")
        .arg("set")
        .arg("Interface")
        .arg(port)
        .arg(&format!("type=internal"))
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("Failed to add internal port to OVS: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
}

fn ovs_del_internal_port(br: &str, vni: u32) {
    let output = std::process::Command::new("ovs-vsctl")
        .arg("del-port")
        .arg(br)
        .arg(&format!("vlink{}", vni))
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("Failed to delete internal port from OVS: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
}

const NETWORK_SVI: &str = r#"
[Match]
Name=svi*

[Network]
DHCP=no
IPv6AcceptRA=no
LinkLocalAddressing=no
"#;

fn netdev_vxlan(vni: u32, local_ip: IpAddr) -> String {
    format!("
[NetDev]
Name=vni{vni}
Kind=vxlan

[VXLAN]
VNI={vni}
DestinationPort=4789
Independent=yes
MacLearning=no
Local={local_ip}
")
}

fn netdev_linux_bridge(vni: u32) -> String {
    format!("
[NetDev]
Kind=bridge
Name=svi{vni}
")
}

fn network_vxlan(vni: u32) -> String {
    format!("
[Match]
Name=vni{vni}

[Network]
IPv6AcceptRA=no
LinkLocalAddressing=no
Bridge=svi{vni}

[Bridge]
Learning=no
NeighborSuppression=yes
UnicastFlood=yes
MulticastFlood=yes
")
}

fn network_vlink(vni: u32) -> String {
    format!("
[Match]
Name=vlink{vni}

[Network]
Bridge=svi{vni}
LinkLocalAddressing=no
")
}

fn enable(config: &config::Config, db: &db::Database, vni: u32) {
    check_for_root();

    let vlan = db.add_vni(vni).unwrap();

    std::fs::write("/etc/systemd/network/vnictl-svi.network", NETWORK_SVI).unwrap(); // it may already exist
    std::fs::write(format!("/etc/systemd/network/vnictl-vni{}.netdev", vni), netdev_vxlan(vni, config.local_ip)).unwrap();
    std::fs::write(format!("/etc/systemd/network/vnictl-vni{}.network", vni), network_vxlan(vni)).unwrap();
    std::fs::write(format!("/etc/systemd/network/vnictl-svi{}.netdev", vni), netdev_linux_bridge(vni)).unwrap();
    std::fs::write(format!("/etc/systemd/network/vnictl-vlink{}.network", vni), network_vlink(vni)).unwrap();

    reload_networkd();
    ovs_add_internal_port(&config.ovs_bridge, vni, vlan);

    println!("Enabled: VNI={} VLAN={}", vni, vlan);
}

fn disable(config: &config::Config, db: &db::Database, vni: u32) {
    check_for_root();

    let vlan = db.remove_vni(vni).unwrap();

    // ignore nonexistent networkd files
    let _ = std::fs::remove_file(format!("/etc/systemd/network/vnictl-vni{}.netdev", vni));
    let _ = std::fs::remove_file(format!("/etc/systemd/network/vnictl-vni{}.network", vni));
    let _ = std::fs::remove_file(format!("/etc/systemd/network/vnictl-svi{}.netdev", vni));
    let _ = std::fs::remove_file(format!("/etc/systemd/network/vnictl-vlink{}.network", vni));

    ovs_del_internal_port(&config.ovs_bridge, vni);
    reload_networkd();

    println!("Disabled: VNI={} VLAN={}", vni, vlan);
}

fn list(_config: &config::Config, db: &db::Database) {
    let vnics = db.list_vni().unwrap();
    for vnic in vnics {
        println!("VNI={} VLAN={}", vnic.vni, vnic.vlan);
    }
}

fn status(_config: &config::Config, db: &db::Database, vni: u32) {
    let vlan = db.get_vlan(vni).unwrap();
    match vlan {
        Some(vlan) => println!("VNI={} VLAN={}", vni, vlan),
        None => {
            eprintln!("VNI={} not found", vni);
            std::process::exit(1);
        },
    }
}
