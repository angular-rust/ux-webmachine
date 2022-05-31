/// Struct to represent an encoding
#[derive(Debug, Clone, PartialEq)]
pub struct Encoding {
    /// Encoding string
    pub encoding: String,
    /// Weight associated with the encoding
    pub weight: f32,
}

impl Encoding {
    /// Parse a string into a Charset struct
    pub fn parse_string(encoding: &str) -> Encoding {
        Encoding {
            encoding: encoding.to_string(),
            weight: 1.0,
        }
    }

    /// Adds a quality weight to the charset
    pub fn with_weight(&self, weight: &str) -> Encoding {
        Encoding {
            encoding: self.encoding.to_string(),
            weight: weight.parse().unwrap_or(1.0),
        }
    }

    /// If this encoding matches the other encoding
    pub fn matches(&self, other: &Encoding) -> bool {
        other.encoding == "*" || (self.encoding.to_lowercase() == other.encoding.to_lowercase())
    }

    /// Converts this encoding into a string
    pub fn to_string(&self) -> String {
        self.encoding.clone()
    }
}
