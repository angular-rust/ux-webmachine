pub(crate) const MAX_STATE_MACHINE_TRANSITIONS: u8 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Decision {
    Start,
    End(u16),
    A3Options,
    B3Options,
    B4RequestEntityTooLarge,
    B5UnknownContentType,
    B6UnsupportedContentHeader,
    B7Forbidden,
    B8Authorized,
    B9MalformedRequest,
    B10MethodAllowed,
    B11UriTooLong,
    B12KnownMethod,
    B13Available,
    C3AcceptExists,
    C4AcceptableMediaTypeAvailable,
    D4AcceptLanguageExists,
    D5AcceptableLanguageAvailable,
    E5AcceptCharsetExists,
    E6AcceptableCharsetAvailable,
    F6AcceptEncodingExists,
    F7AcceptableEncodingAvailable,
    G7ResourceExists,
    G8IfMatchExists,
    G9IfMatchStarExists,
    G11EtagInIfMatch,
    H7IfMatchStarExists,
    H10IfUnmodifiedSinceExists,
    H11IfUnmodifiedSinceValid,
    H12LastModifiedGreaterThanUMS,
    I4HasMovedPermanently,
    I12IfNoneMatchExists,
    I13IfNoneMatchStarExists,
    I7Put,
    J18GetHead,
    K5HasMovedPermanently,
    K7ResourcePreviouslyExisted,
    K13ETagInIfNoneMatch,
    L5HasMovedTemporarily,
    L7Post,
    L13IfModifiedSinceExists,
    L14IfModifiedSinceValid,
    L15IfModifiedSinceGreaterThanNow,
    L17IfLastModifiedGreaterThanMS,
    M5Post,
    M7PostToMissingResource,
    M16Delete,
    M20DeleteEnacted,
    N5PostToMissingResource,
    N11Redirect,
    N16Post,
    O14Conflict,
    O16Put,
    O18MultipleRepresentations,
    O20ResponseHasBody,
    P3Conflict,
    P11NewResource,
}

impl Decision {
    pub(crate) fn is_terminal(&self) -> bool {
        match self {
            &Decision::End(_) => true,
            &Decision::A3Options => true,
            _ => false,
        }
    }
}

pub(crate) enum Transition {
    To(Decision),
    Branch(Decision, Decision),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum DecisionResult {
    True(String),
    False(String),
    StatusCode(u16),
}

impl DecisionResult {
    pub(crate) fn wrap(result: bool, reason: &str) -> DecisionResult {
        if result {
            DecisionResult::True(format!("is: {}", reason))
        } else {
            DecisionResult::False(format!("is not: {}", reason))
        }
    }
}
