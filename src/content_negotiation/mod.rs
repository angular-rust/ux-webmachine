//! The `content_negotiation` module deals with handling media types, languages, charsets and
//! encodings as per [https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html][1].
//! 
//! [1]: https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html


use itertools::Itertools;
use std::cmp::Ordering;

use crate::{context::Request, headers::HeaderValue, Resource};

mod charset;
pub use self::charset::*;

mod encoding;
pub use self::encoding::*;

mod medialanguage;
pub use self::medialanguage::*;

mod mediatype;
pub use self::mediatype::*;

/// Sorts the list of media types by their weights
pub fn sort_media_types(media_types: &Vec<HeaderValue>) -> Vec<HeaderValue> {
    media_types
        .into_iter()
        .cloned()
        .sorted_by(|a, b| {
            let media_a = a.as_media_type().weight();
            let media_b = b.as_media_type().weight();
            let order = media_a
                .0
                .partial_cmp(&media_b.0)
                .unwrap_or(Ordering::Greater);
            if order == Ordering::Equal {
                Ord::cmp(&media_a.1, &media_b.1)
            } else {
                order.reverse()
            }
        })
        .collect()
}

/// Determines if the media types produced by the resource matches the acceptable media types
/// provided by the client. Returns the match if there is one.
pub fn matching_content_type(
    resource: &Resource,
    request: &Request,
) -> Option<String> {
    if request.has_accept_header() {
        let acceptable_media_types = sort_media_types(&request.accept());
        resource
            .produces
            .iter()
            .cloned()
            .cartesian_product(acceptable_media_types.iter())
            .map(|(produced, acceptable)| {
                let acceptable_media_type = acceptable.as_media_type();
                let produced_media_type = MediaType::parse_string(produced);
                (
                    produced_media_type.clone(),
                    acceptable_media_type.clone(),
                    produced_media_type.matches(&acceptable_media_type),
                )
            })
            .sorted_by(|a, b| Ord::cmp(&a.2, &b.2))
            .filter(|val| val.2 != MediaTypeMatch::None)
            .next()
            .map(|result| result.0.to_string())
    } else {
        resource.produces.first().map(|s| s.to_string())
    }
}

/// Sorts the list of media types by weighting
pub fn sort_media_languages(media_languages: &Vec<HeaderValue>) -> Vec<MediaLanguage> {
    media_languages
        .iter()
        .cloned()
        .map(|lang| lang.as_media_language())
        .filter(|lang| lang.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
        .collect()
}

/// Determines if the languages produced by the resource matches the acceptable languages
/// provided by the client. Returns the match if there is one.
pub fn matching_language(
    resource: &Resource,
    request: &Request,
) -> Option<String> {
    if request.has_accept_language_header() && !request.accept_language().is_empty() {
        let acceptable_languages = sort_media_languages(&request.accept_language());
        if resource.languages_provided.is_empty() {
            acceptable_languages.first().map(|lang| lang.to_string())
        } else {
            acceptable_languages
                .iter()
                .cartesian_product(resource.languages_provided.iter())
                .map(|(acceptable_language, produced_language)| {
                    let produced_language = MediaLanguage::parse_string(produced_language);
                    (
                        produced_language.clone(),
                        produced_language.matches(&acceptable_language),
                    )
                })
                .find(|val| val.1)
                .map(|result| result.0.to_string())
        }
    } else if resource.languages_provided.is_empty() {
        Some("*".to_string())
    } else {
        resource.languages_provided.first().map(|s| s.to_string())
    }
}

/// Sorts the list of charsets by weighting as per [https://tools.ietf.org/html/rfc2616#section-14.2][1].
/// Note that ISO-8859-1 is added as a default with a weighting of 1 if not all ready supplied.
/// 
/// [1]: https://tools.ietf.org/html/rfc2616#section-14.2
pub fn sort_media_charsets(charsets: &Vec<HeaderValue>) -> Vec<Charset> {
    let mut charsets = charsets.clone();
    if charsets
        .iter()
        .find(|cs| cs.value == "*" || cs.value.to_uppercase() == "ISO-8859-1")
        .is_none()
    {
        charsets.push(h!("ISO-8859-1"));
    }
    charsets
        .into_iter()
        .map(|cs| cs.as_charset())
        .filter(|cs| cs.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
        .collect()
}

/// Determines if the charsets produced by the resource matches the acceptable charsets
/// provided by the client. Returns the match if there is one.
pub fn matching_charset(
    resource: &Resource,
    request: &Request,
) -> Option<String> {
    if request.has_accept_charset_header() && !request.accept_charset().is_empty() {
        let acceptable_charsets = sort_media_charsets(&request.accept_charset());
        if resource.charsets_provided.is_empty() {
            acceptable_charsets.first().map(|cs| cs.to_string())
        } else {
            acceptable_charsets
                .iter()
                .cartesian_product(resource.charsets_provided.iter())
                .map(|(acceptable_charset, provided_charset)| {
                    let provided_charset = Charset::parse_string(provided_charset);
                    (
                        provided_charset.clone(),
                        provided_charset.matches(&acceptable_charset),
                    )
                })
                .find(|val| val.1)
                .map(|result| result.0.to_string())
        }
    } else if resource.charsets_provided.is_empty() {
        Some("ISO-8859-1".to_string())
    } else {
        resource.charsets_provided.first().map(|s| s.to_string())
    }
}

/// Sorts the list of encodings by weighting as per [https://tools.ietf.org/html/rfc2616#section-14.3][1].
/// Note that identity encoding is awlays added with a weight of 1 if not already present.
/// 
/// [1]: https://tools.ietf.org/html/rfc2616#section-14.3
pub fn sort_encodings(encodings: &Vec<HeaderValue>) -> Vec<Encoding> {
    let mut encodings = encodings.clone();
    if encodings
        .iter()
        .find(|e| e.value == "*" || e.value.to_lowercase() == "identity")
        .is_none()
    {
        encodings.push(h!("identity"));
    }
    encodings
        .into_iter()
        .map(|encoding| encoding.as_encoding())
        .filter(|encoding| encoding.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
        .collect()
}

/// Determines if the encodings supported by the resource matches the acceptable encodings
/// provided by the client. Returns the match if there is one.
pub fn matching_encoding(
    resource: &Resource,
    request: &Request,
) -> Option<String> {
    let identity = Encoding::parse_string("identity");
    if request.has_accept_encoding_header() {
        let acceptable_encodings = sort_encodings(&request.accept_encoding());
        if resource.encodings_provided.is_empty() {
            if acceptable_encodings.contains(&identity) {
                Some("identity".to_string())
            } else {
                None
            }
        } else {
            acceptable_encodings
                .iter()
                .cartesian_product(resource.encodings_provided.iter())
                .map(|(acceptable_encoding, provided_encoding)| {
                    let provided_encoding = Encoding::parse_string(provided_encoding);
                    (
                        provided_encoding.clone(),
                        provided_encoding.matches(&acceptable_encoding),
                    )
                })
                .find(|val| val.1)
                .map(|result| result.0.to_string())
        }
    } else if resource.encodings_provided.is_empty() {
        Some("identity".to_string())
    } else {
        resource.encodings_provided.first().map(|s| s.to_string())
    }
}
