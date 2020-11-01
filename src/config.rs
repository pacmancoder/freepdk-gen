use clap::Clap;

use crate::frequency::Frequency;

#[derive(Clap)]
#[clap(
version = env!("CARGO_PKG_VERSION"),
author = env!("CARGO_PKG_AUTHORS"),
about = "FreePDK peripheral generator",
name = "freepdk-gen"
)]
pub struct AppConfig {
    #[clap(long, short, about = "MCU frequency")]
    pub freq: Frequency,
    #[clap(subcommand)]
    pub subcommand: AppSubcommand,
}

#[derive(Clap)]
pub enum AppSubcommand {
    #[clap(about = "Generate software uart implementation")]
    Uart(UartSubcommand),
}

#[derive(Clap)]
pub struct UartSubcommand {
    #[clap(long, short)]
    pub baud: u32,
}