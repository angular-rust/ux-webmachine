use itertools::Itertools;

/// Struct to represent a media language
#[derive(Debug, Clone, PartialEq)]
pub struct MediaLanguage {
    /// Main type of the media language
    pub main: String,
    /// Sub type of the media language
    pub sub: String,
    /// Weight associated with the media language
    pub weight: f32,
}

impl MediaLanguage {
    /// Parse a string into a MediaLanguage struct
    pub fn parse_string(language: &str) -> MediaLanguage {
        let types: Vec<&str> = language.splitn(2, '-').collect_vec();
        if types.is_empty() || types[0].is_empty() {
            MediaLanguage {
                main: "*".to_string(),
                sub: "".to_string(),
                weight: 1.0,
            }
        } else {
            MediaLanguage {
                main: types[0].to_string(),
                sub: if types.len() == 1 || types[1].is_empty() {
                    "".to_string()
                } else {
                    types[1].to_string()
                },
                weight: 1.0,
            }
        }
    }

    /// Adds a quality weight to the media language
    pub fn with_weight(&self, weight: &str) -> MediaLanguage {
        MediaLanguage {
            main: self.main.clone(),
            sub: self.sub.clone(),
            weight: weight.parse().unwrap_or(1.0),
        }
    }

    /// If this media language matches the other media language
    pub fn matches(&self, other: &MediaLanguage) -> bool {
        if other.main == "*" || (self.main == other.main && self.sub == other.sub) {
            true
        } else {
            let check = format!("{}-", self.to_string());
            other.to_string().starts_with(&check)
        }
    }

    /// Converts this media language into a string
    pub fn to_string(&self) -> String {
        if self.sub.is_empty() {
            self.main.clone()
        } else {
            format!("{}-{}", self.main, self.sub)
        }
    }
}
