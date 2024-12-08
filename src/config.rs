
use serde::Deserialize;

use std::path::PathBuf;

/// Configuration file
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// OVS bridge name to add VNI VLANs to
    pub ovs_bridge: String,

    /// Local IP address to use for VTEP
    pub local_ip: std::net::IpAddr,

    /// Path to the database file
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,

    /// The first VLAN ID to use
    #[serde(default = "default_start_vlan")]
    pub start_vlan: u16,
}

fn default_db_path() -> PathBuf {
    "/var/lib/vnictl/db.sqlite3".into()
}

fn default_start_vlan() -> u16 {
    2000
}
