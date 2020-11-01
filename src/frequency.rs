use std::str::FromStr;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Frequency(u32);

impl Into<u32> for Frequency {
    fn into(self) -> u32 {
        return self.0
    }
}

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