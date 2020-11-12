use clap::Clap;

use crate::mcu::{Frequency, Port, Pin};

#[derive(Clap)]
#[clap(
version = env!("CARGO_PKG_VERSION"),
author = env!("CARGO_PKG_AUTHORS"),
about = env!("CARGO_PKG_DESCRIPTION"),
name = env!("CARGO_PKG_NAME")
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
    #[clap(long, about = "Sets generated UART baud rate")]
    pub baud: u32,
    #[clap(long, about = "Port to use for UART TX pin")]
    pub tx_port: Port,
    #[clap(long, about = "Pin to use for UART TX")]
    pub tx_pin: Pin,
    #[clap(long, about = "Invert UART TX logic level")]
    pub invert_tx: bool,
    #[clap(long, about = "Customize generated UART TX function name")]
    pub tx_function_name: Option<String>,
}