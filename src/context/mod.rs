//! The `context` module encapsulates the context of the environment that the webmachine is
//! executing in. Basically wraps the request and response.

use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;

mod request;
pub use self::request::*;

mod response;
pub use self::response::*;

/// Main context struct that holds the request and response.
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    /// Request that the webmachine is executing against
    pub request: Request,
    /// Response that is the result of the execution
    pub response: Response,
    /// selected media type after content negotiation
    pub selected_media_type: Option<String>,
    /// selected language after content negotiation
    pub selected_language: Option<String>,
    /// selected charset after content negotiation
    pub selected_charset: Option<String>,
    /// selected encoding after content negotiation
    pub selected_encoding: Option<String>,
    /// parsed date and time from the If-Unmodified-Since header
    pub if_unmodified_since: Option<DateTime<FixedOffset>>,
    /// parsed date and time from the If-Modified-Since header
    pub if_modified_since: Option<DateTime<FixedOffset>>,
    /// If the response should be a redirect
    pub redirect: bool,
    /// If a new resource was created
    pub new_resource: bool,
    /// General store of metadata. You can use this to store attributes as the webmachine executes.
    pub metadata: HashMap<String, String>,
}

impl Default for Context {
    /// Creates a default context
    fn default() -> Context {
        Context {
            request: Request::default(),
            response: Response::default(),
            selected_media_type: None,
            selected_language: None,
            selected_charset: None,
            selected_encoding: None,
            if_unmodified_since: None,
            if_modified_since: None,
            redirect: false,
            new_resource: false,
            metadata: HashMap::new(),
        }
    }
}
