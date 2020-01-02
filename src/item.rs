use lotp::totp;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Digits {
    Six,
    Seven,
    Eight,
}

impl std::fmt::Display for Digits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Digits::Six => return write!(f, "6"),
            Digits::Seven => return write!(f, "7"),
            Digits::Eight => return write!(f, "8"),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Item {
    pub label: String,
    pub secret: String,
    pub digits: Digits,
    pub split_time: u32,
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(
            f,
            "({}, {}, {}, {})",
            self.label, self.secret, self.digits, self.split_time
        );
    }
}

impl Item {
    pub fn get_code(&self) -> Result<String, String> {
        match &self.digits {
            Digits::Six => {
                match totp::generate_6_digit_totp_string(&self.secret, &(self.split_time as u64)) {
                    Ok(s) => return Ok(s),
                    Err(e) => return Err(e.description()),
                }
            }
            Digits::Seven => {
                match totp::generate_7_digit_totp_string(&self.secret, &(self.split_time as u64)) {
                    Ok(s) => return Ok(s),
                    Err(e) => return Err(e.description()),
                }
            }
            Digits::Eight => {
                match totp::generate_8_digit_totp_string(&self.secret, &(self.split_time as u64)) {
                    Ok(s) => return Ok(s),
                    Err(e) => return Err(e.description()),
                }
            }
        }
    }
}
