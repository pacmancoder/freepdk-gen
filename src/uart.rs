use thiserror::Error;
use log::info;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::{
    mcu::{Frequency, Port, Pin},
    config::{AppConfig, AppSubcommand},
};
use crate::mcu::StopBits;

const DEFAULT_MAX_CLOCK_DERIVATION: f64 = 0.01;
const MAX_CLOCKS_PER_BIT: u32 = 256 * 4;
const MIN_CLOCKS_PER_BIT: u32 = 16;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid generator options")]
    InvalidOptions,
    #[error("{} clocks per bit is not implemented yet. Max allowed clocks per bit are {}, try lower frequency or higher baud rate", _0, MAX_CLOCKS_PER_BIT)]
    TooManyClocksPerBit(u32),
    #[error("Clock derivation is higher than allowed {:.2}%", _0 * 100f64)]
    TooBigClockDerivation(f64),
    #[error("Calculated clocks per bit ({}) are too small (more than {} is required), try higher frequency or lower baud rate", _0, MIN_CLOCKS_PER_BIT)]
    VeryFewClocksPerBit(u32),
    #[error("Template rendering failed: {}", _0)]
    TemplateFailure(String),
}

impl From<tinytemplate::error::Error> for Error {
    fn from(e: tinytemplate::error::Error) -> Self {
        Self::TemplateFailure(format!("{}", e))
    }
}

#[derive(Default)]
pub struct UartGeneratorBuilder {
    frequency: Option<Frequency>,
    baud: Option<u32>,
    tx_pin: Option<Pin>,
    tx_port: Option<Port>,
    max_clock_derivation: Option<f64>,
    invert_tx: bool,
    tx_function_name: Option<String>,
    stop_bits: Option<StopBits>,
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
        self.tx_port.replace(uart.tx_port);
        self.tx_pin.replace(uart.tx_pin);
        self.invert_tx = uart.invert_tx;
        self.tx_function_name = uart.tx_function_name.clone();
        self.stop_bits.replace(uart.stop_bits);
        Ok(self)
    }

    fn validate_all_params_specified(&self) -> Result<(), Error> {
        self.frequency.ok_or(Error::InvalidOptions)?;
        self.baud.ok_or(Error::InvalidOptions)?;
        Ok(())
    }

    pub fn build(self) -> Result<UartGenerator, Error> {
        self.validate_all_params_specified()?;

        let frequency = self.frequency.expect("Frequency should be specified");
        let baud = self.baud.expect("Baud rate should be specified");
        let tx_port = self.tx_port.expect("Port should be specified");
        let tx_pin = self.tx_pin.expect("Pin should be specified");
        let max_clock_rate_derivation = self.max_clock_derivation
            .unwrap_or(DEFAULT_MAX_CLOCK_DERIVATION);
        let tx_function_name = self.tx_function_name
            .unwrap_or_else(|| String::from("uart0_send_byte"));

        let expected_clocks_per_bit = (frequency.hz() as f64) / baud as f64;
        let clocks_per_bit = expected_clocks_per_bit.round() as u32;

        info!("Estimated clocks per bit: {}", clocks_per_bit);
        let bit_period = clocks_per_bit as f64 / frequency.hz() as f64;
        info!("Bit period: {:.4}ms ({:.4}us)", bit_period * 1000f64, bit_period * 1000000f64);

        if  clocks_per_bit > MAX_CLOCKS_PER_BIT {
            return Err(Error::TooManyClocksPerBit(clocks_per_bit))
        }
        if clocks_per_bit < MIN_CLOCKS_PER_BIT {
            return Err(Error::VeryFewClocksPerBit(clocks_per_bit))
        }

        let clock_derivation = (clocks_per_bit as f64 - expected_clocks_per_bit as f64).abs()
            / expected_clocks_per_bit;

        info!("Clock rate derivation due to rounding error: {:.2}%", clock_derivation * 100f64);

        if clock_derivation > max_clock_rate_derivation {
            return Err(Error::TooBigClockDerivation(max_clock_rate_derivation));
        }

        let clocks_per_stop_bit = match self.stop_bits.unwrap_or(StopBits::One) {
            StopBits::One => clocks_per_bit,
            StopBits::Two => clocks_per_bit * 2,
            StopBits::OneAndHalf => clocks_per_bit + clocks_per_bit / 2,
        };

        Ok(UartGenerator {
            frequency,
            baud,
            clocks_per_bit,
            clocks_per_stop_bit,
            tx_port,
            tx_pin,
            tx_function_name,
            invert_tx: self.invert_tx,
        })
    }
}

#[derive(Serialize)]
struct TemplateContext {
    app_name: &'static str,
    app_version: &'static str,

    frequency: u32,
    baud: u32,

    tx_function_name: String,
    tx_port: char,
    tx_pin: u8,
    tx_inverted: bool,
    tx_start_bit_wait_cycles: u32,
    tx_start_bit_tail_wait_instructions: Vec<&'static str>,
    tx_bit_wait_cycles: u32,
    tx_bit_tail_wait_instructions: Vec<&'static str>,
    tx_stop_bit_wait_cycles: u32,
    tx_stop_bit_tail_wait_instructions: Vec<&'static str>,
}

const UART_TEMPLATE: &str = r##"// THIS FILE WAS GENERATED BY {app_name} v{app_version}
// Target F_CPU: {frequency};  Target baud: {baud}
// TX pin: P{tx_port}{tx_pin}; TX Inverted: {tx_inverted}
#include <stdint.h>
#include <pdk/device.h>

#ifndef F_CPU
    #error "Generated uart required F_CPU to be set"
#endif

#if F_CPU != {frequency}
    #error "Defined F_CPU does not match generated uart's frequency ({frequency})"
#endif

static uint8_t _gen_{tx_function_name}_bits_left;

static void {tx_function_name}(uint8_t byte) \{
    __asm
    ; start bit
    {{if tx_inverted}}set1{{else}}set0{{endif}} P{tx_port}_ADDR, #{tx_pin} ; 1T
    mov a, #{tx_start_bit_wait_cycles} ; 1T
    0001$: ; wait loop takes ({tx_start_bit_wait_cycles} * 4 - 1)T
    nop ; 1T
    dzsn a ; Normally 1T, 2T in last cycle
    goto 0001$ ; 2T
    mov a, #8 ; 1T
    mov __gen_{tx_function_name}_bits_left, a ; 1T
    {{for instruction in tx_start_bit_tail_wait_instructions}}{instruction}
    {{endfor}}

    ; send 1 bit; compare (0002$ -- 0004$) will take 8T
    0002$:
    sr _{tx_function_name}_PARM_1 ; 1T, carry flag will contain LSB
    t1sn f, c ; 1T when bit is 0, in other case - 2T
    goto .+4 ; 2T
    nop ; 1T
    {{if tx_inverted}}set0{{else}}set1{{endif}} P{tx_port}_ADDR, #{tx_pin} ; 1T
    goto .+3 ; 2T
    {{if tx_inverted}}set1{{else}}set0{{endif}} P{tx_port}_ADDR, #{tx_pin} ; 1T
    goto .+1 ; 2T goto isntead of nop to equalify branches
    mov a, #{tx_bit_wait_cycles} ; 1T
    0004$: ; wait loop takes ({tx_bit_wait_cycles} * 4 - 1)T
    nop ; 1T
    dzsn a ; 1T normally, 2T on skip
    goto 0004$ ; 2T
    {{for instruction in tx_bit_tail_wait_instructions}}{instruction}
    {{endfor}}

    ; check for more bits; following chunk will take 3T in any case
    dzsn __gen_{tx_function_name}_bits_left ; 1T normally, 2T on skip
    goto 0002$ ; 2T
    nop ; 1T

    ; wait + 5T to adjust lag from the code above
    goto .+1 ; 2T
    goto .+1 ; 2T
    nop ; 1T

    ; send stop bit
    {{if tx_inverted}}set0{{else}}set1{{endif}} P{tx_port}_ADDR, #{tx_pin} ; 1T
    MOV a, #15 ; 1T
    0005$: ; wait loop takes ({tx_stop_bit_wait_cycles} * 4 - 1)
    nop ; 1T
    dzsn a ; 1T normally, 2T on skip
    goto 0005$ ; 2T
    {{for instruction in tx_stop_bit_tail_wait_instructions}}{instruction}
    {{endfor}}
    __endasm;
}
"##;

pub struct UartGenerator {
    frequency: Frequency,
    baud: u32,
    clocks_per_bit: u32,
    clocks_per_stop_bit: u32,
    tx_port: Port,
    tx_pin: Pin,
    tx_function_name: String,
    invert_tx: bool,
}

fn generate_space_optimal_nop_chain(count: u32) -> Vec<&'static str> {
    match count {
        0 => vec![],
        1 => vec!["nop ; 1T"],
        2 => vec!["goto .+1 ; 2T"],
        3 => vec!["goto .+1 ; 2T", "nop ; 1T"],
        _ => panic!("Function designed to work only with 4T wait loops"),
    }
}

impl UartGenerator {
    pub fn builder() -> UartGeneratorBuilder {
        UartGeneratorBuilder::default()
    }

    pub fn generate(&self) -> Result<(), Error> {
        const WAIT_LOOP_MISSING_LOCKS: u32 = 1;
        const TX_SET_WAIT_LOOP_COUNTER_CLOCKS: u32 = 1;
        const TX_SET_PIN_CLOCKS: u32 = 1;

        const TX_BIT_SET_LOOP_LAG_CLOCKS: u32 = 5;
        const TX_RESET_BIT_COUNTER_CLOCKS: u32 = 2;

        let tx_start_bit_wait_clocks = self.clocks_per_bit
            - TX_BIT_SET_LOOP_LAG_CLOCKS
            - TX_SET_WAIT_LOOP_COUNTER_CLOCKS
            - TX_SET_PIN_CLOCKS
            - TX_RESET_BIT_COUNTER_CLOCKS
            + WAIT_LOOP_MISSING_LOCKS;

        let tx_start_bit_wait_cycles = tx_start_bit_wait_clocks / 4;
        let tx_start_bit_tail_wait_cycles = tx_start_bit_wait_clocks % 4;
        let tx_start_bit_tail_wait_instructions =
            generate_space_optimal_nop_chain(tx_start_bit_tail_wait_cycles);

        const TX_BIT_COMPARE_AND_SET_PIN_CLOCKS: u32 = 8;
        const TX_COMPARE_BIT_COUNT_CLOCKS: u32 = 3;

        let tx_bit_wait_clocks = self.clocks_per_bit
            - TX_BIT_COMPARE_AND_SET_PIN_CLOCKS
            - TX_SET_WAIT_LOOP_COUNTER_CLOCKS
            - TX_COMPARE_BIT_COUNT_CLOCKS
            + WAIT_LOOP_MISSING_LOCKS;

        let tx_bit_wait_cycles = tx_bit_wait_clocks / 4;
        let tx_bit_tail_wait_cycles = tx_bit_wait_clocks % 4;
        let tx_bit_tail_wait_instructions =
            generate_space_optimal_nop_chain(tx_bit_tail_wait_cycles);

        let tx_stop_bit_wait_clocks = self.clocks_per_stop_bit
            - TX_BIT_SET_LOOP_LAG_CLOCKS
            - TX_SET_PIN_CLOCKS
            - TX_SET_WAIT_LOOP_COUNTER_CLOCKS
            + WAIT_LOOP_MISSING_LOCKS;

        let tx_stop_bit_wait_cycles = tx_stop_bit_wait_clocks / 4;
        let tx_stop_bit_tail_wait_cycles = tx_stop_bit_wait_clocks % 4;
        let tx_stop_bit_tail_wait_instructions =
            generate_space_optimal_nop_chain(tx_stop_bit_tail_wait_cycles);

        let context = TemplateContext {
            app_name: env!("CARGO_PKG_NAME"),
            app_version: env!("CARGO_PKG_VERSION"),

            frequency: self.frequency.hz(),
            baud: self.baud,

            tx_function_name: self.tx_function_name.clone(),
            tx_port: self.tx_port.char(),
            tx_pin: self.tx_pin.num(),
            tx_inverted: self.invert_tx,
            tx_start_bit_wait_cycles,
            tx_start_bit_tail_wait_instructions,
            tx_bit_wait_cycles,
            tx_bit_tail_wait_instructions,
            tx_stop_bit_wait_cycles,
            tx_stop_bit_tail_wait_instructions,
        };

        let mut renderer = TinyTemplate::new();
        renderer.add_template("uart", UART_TEMPLATE)?;
        let rendered = renderer.render("uart", &context)?;
        println!("Rendered: \n{}", rendered);
        Ok(())
    }
}