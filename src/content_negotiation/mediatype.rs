use itertools::Itertools;

/// Enum to represent a match with media types
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaTypeMatch {
    /// Full match
    Full,
    /// Match where the sub-type was a wild card
    SubStar,
    /// Full whild card match (type and sub-type)
    Star,
    /// Does not match
    None,
}

/// Struct to represent a media type
#[derive(Debug, Clone, PartialEq)]
pub struct MediaType {
    /// Main type of the media type
    pub main: String,
    /// Sub type of the media type
    pub sub: String,
    /// Weight associated with the media type
    pub weight: f32,
}

impl MediaType {
    /// Parse a string into a MediaType struct
    pub fn parse_string(media_type: &str) -> MediaType {
        let types: Vec<&str> = media_type.splitn(2, '/').collect_vec();
        if types.is_empty() || types[0].is_empty() {
            MediaType {
                main: "*".to_string(),
                sub: "*".to_string(),
                weight: 1.0,
            }
        } else {
            MediaType {
                main: types[0].to_string(),
                sub: if types.len() == 1 || types[1].is_empty() {
                    "*".to_string()
                } else {
                    types[1].to_string()
                },
                weight: 1.0,
            }
        }
    }

    /// Adds a quality weight to the media type
    pub fn with_weight(&self, weight: &String) -> MediaType {
        MediaType {
            main: self.main.clone(),
            sub: self.sub.clone(),
            weight: weight.parse().unwrap_or(1.0),
        }
    }

    /// Returns a weighting for this media type
    pub fn weight(&self) -> (f32, u8) {
        if self.main == "*" && self.sub == "*" {
            (self.weight, 2)
        } else if self.sub == "*" {
            (self.weight, 1)
        } else {
            (self.weight, 0)
        }
    }

    /// If this media type matches the other media type
    pub fn matches(&self, other: &MediaType) -> MediaTypeMatch {
        if other.main == "*" {
            MediaTypeMatch::Star
        } else if self.main == other.main && other.sub == "*" {
            MediaTypeMatch::SubStar
        } else if self.main == other.main && self.sub == other.sub {
            MediaTypeMatch::Full
        } else {
            MediaTypeMatch::None
        }
    }

    /// Converts this media type into a string
    pub fn to_string(&self) -> String {
        format!("{}/{}", self.main, self.sub)
    }
}
