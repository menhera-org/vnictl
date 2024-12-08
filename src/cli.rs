
use clap::{Parser, Subcommand};

use std::path::PathBuf;

/// EVPN VxLAN VNI manager
#[derive(Debug, Clone, Parser)]
#[clap(name = "vnictl", version, about)]
pub struct Cli {
    /// Path to the configuration file
    #[clap(short, long, default_value = "/etc/vnictl.toml")]
    pub config: PathBuf,

    #[clap(subcommand)]
    pub subcmd: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Add a VNI to the local bridge
    #[command(arg_required_else_help = true)]
    Enable {
        vni: u32,
    },

    /// Remove a VNI from the local bridge
    #[command(arg_required_else_help = true)]
    Disable {
        vni: u32,
    },

    /// List all VNIs on the local bridge
    List,

    /// Show the status of a VNI on the local bridge
    #[command(arg_required_else_help = true)]
    Status {
        vni: u32,
    },
}
