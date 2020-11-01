use thiserror::Error;
use log::info;

use crate::{
    frequency::Frequency,
    config::{AppConfig, AppSubcommand},
};

const DEFAULT_MAX_CLOCK_DERIVATION: f64 = 0.01;
const MAX_CLOCKS_PER_BIT: u32 = 256 * 4;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid generator options")]
    InvalidOptions,
    #[error("More than {} clocks per bit is not implemented yet, try higher frequency or higher baud rate", MAX_CLOCKS_PER_BIT)]
    TooManyClocksPerBit,
    #[error("Clock derivation is higher than allowed {:.2}%", _0 * 100f64)]
    TooBigClockDerivation(f64),
}

#[derive(Default)]
pub struct UartGeneratorBuilder {
    frequency: Option<Frequency>,
    baud: Option<u32>,
    max_clock_derivation: Option<f64>,
}

impl UartGeneratorBuilder {
    pub fn load_config(mut self, config: &AppConfig) -> Result<Self, Error> {
        #[allow(unreachable_patterns)]
        let uart = match &config.subcommand {
            AppSubcommand::Uart(command) => command,
            _ => panic!("UartGenerator::from_config should called only when uart subcommand is active"),
        };

        self.frequency.replace(config.freq);
        self.baud.replace(uart.baud);
        Ok(self)
    }

    fn validate_all_params_specified(&self) -> Result<(), Error> {
        self.frequency.ok_or(Error::InvalidOptions)?;
        self.baud.ok_or(Error::InvalidOptions)?;
        Ok(())
    }

    pub fn build(self) -> Result<UartGenerator, Error> {
        self.validate_all_params_specified()?;

        let frequency = self.frequency.unwrap();
        let baud = self.baud.unwrap();
        let max_clock_rate_derivation = self.max_clock_derivation
            .unwrap_or(DEFAULT_MAX_CLOCK_DERIVATION);

        let expected_clocks_per_bit = (frequency.hz() as f64) / baud as f64;
        let clocks_per_bit = expected_clocks_per_bit.round() as u32;

        info!("Estimated clocks per bit: {}", clocks_per_bit);
        let bit_period = clocks_per_bit as f64 / frequency.hz() as f64;
        info!("Bit period: {:.4}ms ({:.4}us)", bit_period * 1000f64, bit_period * 1000000f64);

        if  clocks_per_bit > MAX_CLOCKS_PER_BIT {
            return Err(Error::TooManyClocksPerBit)
        }

        let clock_derivation = (clocks_per_bit as f64 - expected_clocks_per_bit as f64).abs()
            / expected_clocks_per_bit;

        info!("Clock rate derivation due to rounding error: {:.2}%", clock_derivation * 100f64);

        if clock_derivation > max_clock_rate_derivation {
            return Err(Error::TooBigClockDerivation(max_clock_rate_derivation));
        }

        Ok(UartGenerator {
            frequency,
            baud,
            clocks_per_bit,
        })
    }
}

pub struct UartGenerator {
    frequency: Frequency,
    baud: u32,
    clocks_per_bit: u32,
}

impl UartGenerator {
    pub fn builder() -> UartGeneratorBuilder {
        UartGeneratorBuilder::default()
    }

    pub fn generate(&self) -> Result<(), Error> {
        Ok(())
    }
}
