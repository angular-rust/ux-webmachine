/// Struct to represent a character set
#[derive(Debug, Clone, PartialEq)]
pub struct Charset {
    /// Charset code
    pub charset: String,
    /// Weight associated with the charset
    pub weight: f32,
}

impl Charset {
    /// Parse a string into a Charset struct
    pub fn parse_string(charset: &str) -> Charset {
        Charset {
            charset: charset.to_string(),
            weight: 1.0,
        }
    }

    /// Adds a quality weight to the charset
    pub fn with_weight(&self, weight: &str) -> Charset {
        Charset {
            charset: self.charset.clone(),
            weight: weight.parse().unwrap_or(1.0),
        }
    }

    /// If this media charset matches the other media charset
    pub fn matches(&self, other: &Charset) -> bool {
        other.charset == "*" || (self.charset.to_uppercase() == other.charset.to_uppercase())
    }

    /// Converts this charset into a string
    pub fn to_string(&self) -> String {
        self.charset.clone()
    }
}
