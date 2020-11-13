use std::str::FromStr;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Frequency(u32);

impl Display for Frequency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Frequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.find(|ch: char| !ch.is_digit(10)) {
            None => {
                let value: u32 = s.parse().map_err(|_| "Frequency is not a number".to_string())?;
                Ok(Self(value))
            },
            Some(split_pos) => {
                let (value_str, multiplier_str) = s.split_at(split_pos);
                let value: u32 = value_str
                    .parse()
                    .map_err(|_| "Frequency is not a number".to_string())?;

                let multiplier = match multiplier_str.to_lowercase().as_str() {
                    "hz" => 1,
                    "khz" => 1000,
                    "mhz" => 1000000,
                    _ => return Err("Invalid frequency suffix".into()),
                };

                match value.overflowing_mul(multiplier) {
                    (value, false) => Ok(Self(value)),
                    (_, true) => Err("Frequency is too big".into()),
                }
            }
        }
    }
}

impl Frequency {
    pub fn hz(self) -> u32 {
        self.0
    }
}

// At the moment of writing this, on padauk devices only A, B and C ports were available
const PORTS: [char; 3] = ['A', 'B', 'C'];

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Port(char);

impl FromStr for Port {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let available_ports_str = PORTS.iter().fold(String::new(), |mut str, ch| {
            str.push(*ch);
            str
        } );

        if s.len() != 1 {
            return Err(format!(
                "Port should be represented as a single character ({})",
                available_ports_str
            ));
        }

        let normalized_port = s.chars().next().unwrap().to_ascii_uppercase();

        if !PORTS.contains(&normalized_port) {
            return Err("Unknown port".into());
        }

        Ok(Port(normalized_port))
    }
}

impl Port {
    pub fn char(&self) -> char {
        self.0
    }
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Pin(u8);

impl FromStr for Pin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s.parse().map_err(|_| "Invalid pin number".to_string())?;
        if value > 7 as u8 {
            Err("Pin can't be bigger than 7".to_string())
        } else {
            Ok(Self(value))
        }
    }
}

impl Pin {
    pub fn num(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum StopBits {
    One,
    Two,
    OneAndHalf,
}

impl FromStr for StopBits {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::One),
            "2" => Ok(Self::Two),
            "1.5" => Ok(Self::OneAndHalf),
            _ => Err("Invalid stop bits value".to_string())
        }
    }
}

