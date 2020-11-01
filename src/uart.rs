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

/*
DRAFT clock-perfect uart tx (bound to 8MHz & 115200 baud rate)

static uint8_t tx_bit_count;

// calculation for 69T bit length (115200 baud & 8MHz clock)

// Start bit: (69 - 5(lag) - 1(set pin) - 1(set loop var) - 2 (reset bit counter)) wait clocks = 60T
// nearest N for 4-wait clock cycle: 15 (59 clocks)
// 1 NOP's required

// Bits loop: each loop 6T comp&set and 3T bit switch + 1T set wait loop value (10T)
// wait loop: 69 - 9 = 59T
// nearest N for 4-wait clock cycle: 15 (59 clocks)
//  0 NOP's required

// 7T compare + 1T set wait loop value + 3T bit check -> 11T
// 69 - 11 = 58
// N = 14 for 56T; 2 NOP required

// Stop bit: 69 - 5(lag) - 1(set) - 1(set wait loop value) = 62
// nearest N for 4-wait clock cycle: 15 (59 clocks)
// 3 NOP's required

static void uart_send_byte(uint8_t byte) {
    __asm
        set0 PA_ADDR, #0        ; 1CLK
        mov a, #15              ; 1CLK; (we need to wait 4 clock less(to accomodate lag in the following block - 1 cycle in set0))
        0001$:                  ; Start bit; wait loop takes (N * 4 - 1)
        nop                     ; 1CLK
        dzsn a                  ; 1CLK / 2CLK SKIP
        goto 0001$              ; 2CLK
        mov a, #8
        mov _tx_bit_count, a
        ; %START_BIT_NOP_BEGIN%
        nop
        ; %START_BIT_NOP_END%

        0002$: ; 8 bit transmittion
        ; LSB bit will be moved to carry
        ; In any case the following snippet will always take 7 clocks (unitl 0004$)
        ; bit will be set on clock 5
        SR _uart_send_byte_PARM_1 ; 1 CLK
        T1SN f, c                 ; 1 CLK : If bit is 0, else - 2
        goto 0003$                ; 2 CLK
        nop;                      ; 1 CLK
        set1 PA_ADDR, #0          ; 1 CLK
        goto .+3                  ; 2 CLK
        0003$:
        set0 PA_ADDR, #0          ; 1 CLK
        goto .+1                  ; 2 CLK to equalify branches

        mov a, #14              ; 1CLK; 17 * 4 + 1
        0004$:
        nop                     ; 1CLK
        dzsn a                  ; 1CLK / 2CLK SKIP
        goto 0004$              ; 2CLK
        ; %BIT_NOP_BEGIN%
        nop
        nop
        ; %BIT_NOP_END%

        ; block below always takes 3 T
        dzsn _tx_bit_count      ; 1CLK / 2CLK SKIP
        goto 0002$              ; 2CLK
        nop                     ; 1T

        ; We need to wait + 5T to accomodate lag in bit setting above
        goto .+1
        goto .+1
        nop
        set1 PA_ADDR, #0

        MOV a, #15              ; 1CLK; (we need to wait 4 clock less(to accomodate lag in the following block - 1 cycle in set0))
        0005$:                  ; Start bit; wait loop takes (N * 4 - 1)
        nop                     ; 1CLK
        dzsn a                  ; 1CLK / 2CLK SKIP
        goto 0005$              ; 2CLK
        ; %STOP_BIT_NOP_BEGIN%
        nop
        nop
        nop
        ; %STOP_BIT_NOP_END%
    __endasm;
}
 */