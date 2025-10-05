use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "cursedboard")]
#[command(about = "Cross-platform clipboard synchronization", long_about = None)]
pub struct Cli {
    #[arg(long, help = "Disable automatic peer discovery")]
    pub no_discovery: bool,

    #[arg(long, help = "Group name for peer filtering (default: username)")]
    pub group: Option<String>,

    #[arg(long, help = "Pre-shared key for authentication")]
    pub psk: Option<String>,

    #[arg(long, help = "Enable pairing mode for N seconds (accepts first new peer)")]
    pub pair: Option<u64>,

    #[arg(long, default_value = "34254", help = "Port to listen on")]
    pub port: u16,

    #[arg(long, help = "Path to config file")]
    pub config: Option<String>,

    #[arg(short, long, help = "Log level (trace, debug, info, warn, error)")]
    pub log_level: Option<String>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
