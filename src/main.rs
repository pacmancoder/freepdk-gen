use clap::Clap;
use anyhow::Error;

use freepdk_gen::{
    config::{AppConfig, AppSubcommand},
    uart::UartGenerator
};

fn main() -> Result<(), Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    let config: AppConfig = AppConfig::parse();

    if matches!(config.subcommand, AppSubcommand::Uart(_)) {
        let generated_data = UartGenerator::builder()
            .load_config(&config)?
            .build()?
            .generate()?;

        println!("Generated file:\n{0}", generated_data)
    }

    Ok(())
}
