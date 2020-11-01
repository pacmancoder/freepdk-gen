mod frequency;
mod config;
mod uart;

use clap::Clap;
use anyhow::Error;

use crate::config::{AppConfig, AppSubcommand};

fn main() -> Result<(), Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    let config: AppConfig = AppConfig::parse();

    if matches!(config.subcommand, AppSubcommand::Uart(_)) {
        uart::UartGenerator::builder()
            .load_config(&config)?
            .build()?
            .generate()?;
    }

    Ok(())
}
