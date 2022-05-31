use std::collections::HashMap;

use crate::headers::HeaderValue;

/// Request that the state machine is executing against
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Path of the request relative to the resource
    pub request_path: String,
    /// Resource base path
    pub base_path: String,
    /// Request method
    pub method: String,
    /// Request headers
    pub headers: HashMap<String, Vec<HeaderValue>>,
    /// Request body
    pub body: Option<Vec<u8>>,
    /// Query parameters
    pub query: HashMap<String, Vec<String>>,
}

impl Default for Request {
    /// Creates a default request (GET /)
    fn default() -> Request {
        Request {
            request_path: "/".to_string(),
            base_path: "/".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
        }
    }
}

impl Request {
    /// returns the content type of the request, based on the content type header. Defaults to
    /// 'application/json' if there is no header.
    pub fn content_type(&self) -> String {
        match self
            .headers
            .keys()
            .find(|k| k.to_uppercase() == "CONTENT-TYPE")
        {
            Some(header) => match self.headers.get(header).unwrap().first() {
                Some(value) => value.clone().value,
                None => "application/json".to_string(),
            },
            None => "application/json".to_string(),
        }
    }

    /// If the request is a put or post
    pub fn is_put_or_post(&self) -> bool {
        ["PUT", "POST"].contains(&self.method.to_uppercase().as_str())
    }

    /// If the request is a get or head request
    pub fn is_get_or_head(&self) -> bool {
        ["GET", "HEAD"].contains(&self.method.to_uppercase().as_str())
    }

    /// If the request is a get
    pub fn is_get(&self) -> bool {
        self.method.to_uppercase() == "GET"
    }

    /// If the request is an options
    pub fn is_options(&self) -> bool {
        self.method.to_uppercase() == "OPTIONS"
    }

    /// If the request is a put
    pub fn is_put(&self) -> bool {
        self.method.to_uppercase() == "PUT"
    }

    /// If the request is a post
    pub fn is_post(&self) -> bool {
        self.method.to_uppercase() == "POST"
    }

    /// If the request is a delete
    pub fn is_delete(&self) -> bool {
        self.method.to_uppercase() == "DELETE"
    }

    /// If an Accept header exists
    pub fn has_accept_header(&self) -> bool {
        self.has_header("ACCEPT")
    }

    /// Returns the acceptable media types from the Accept header
    pub fn accept(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT")
    }

    /// If an Accept-Language header exists
    pub fn has_accept_language_header(&self) -> bool {
        self.has_header("ACCEPT-LANGUAGE")
    }

    /// Returns the acceptable languages from the Accept-Language header
    pub fn accept_language(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-LANGUAGE")
    }

    /// If an Accept-Charset header exists
    pub fn has_accept_charset_header(&self) -> bool {
        self.has_header("ACCEPT-CHARSET")
    }

    /// Returns the acceptable charsets from the Accept-Charset header
    pub fn accept_charset(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-CHARSET")
    }

    /// If an Accept-Encoding header exists
    pub fn has_accept_encoding_header(&self) -> bool {
        self.has_header("ACCEPT-ENCODING")
    }

    /// Returns the acceptable encodings from the Accept-Encoding header
    pub fn accept_encoding(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-ENCODING")
    }

    /// If the request has the provided header
    pub fn has_header(&self, header: &str) -> bool {
        self.headers
            .keys()
            .find(|k| k.to_uppercase() == header.to_uppercase())
            .is_some()
    }

    /// Returns the list of values for the provided request header. If the header is not present,
    /// or has no value, and empty vector is returned.
    pub fn find_header(&self, header: &str) -> Vec<HeaderValue> {
        match self
            .headers
            .keys()
            .find(|k| k.to_uppercase() == header.to_uppercase())
        {
            Some(header) => self.headers.get(header).unwrap().clone(),
            None => Vec::new(),
        }
    }

    /// If the header has a matching value
    pub fn has_header_value(&self, header: &str, value: &str) -> bool {
        match self
            .headers
            .keys()
            .find(|k| k.to_uppercase() == header.to_uppercase())
        {
            Some(header) => match self
                .headers
                .get(header)
                .unwrap()
                .iter()
                .find(|val| *val == value)
            {
                Some(_) => true,
                None => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers::*;
    use expectest::prelude::*;

    #[test]
    fn request_does_not_have_header_test() {
        let request = Request {
            ..Request::default()
        };
        expect!(request.has_header("Vary")).to(be_false());
        expect!(request.has_header_value("Vary", "*")).to(be_false());
    }

    #[test]
    fn request_with_empty_header_test() {
        let request = Request {
            headers: hashmap! { "HeaderA".to_string() => Vec::new() },
            ..Request::default()
        };
        expect!(request.has_header("HeaderA")).to(be_true());
        expect!(request.has_header_value("HeaderA", "*")).to(be_false());
    }

    #[test]
    fn request_with_header_single_value_test() {
        let request = Request {
            headers: hashmap! { "HeaderA".to_string() => vec![h!("*")] },
            ..Request::default()
        };
        expect!(request.has_header("HeaderA")).to(be_true());
        expect!(request.has_header_value("HeaderA", "*")).to(be_true());
        expect!(request.has_header_value("HeaderA", "other")).to(be_false());
    }

    #[test]
    fn request_with_header_multiple_value_test() {
        let request = Request {
            headers: hashmap! { "HeaderA".to_string() => vec![h!("*"), h!("other")]},
            ..Request::default()
        };
        expect!(request.has_header("HeaderA")).to(be_true());
        expect!(request.has_header_value("HeaderA", "*")).to(be_true());
        expect!(request.has_header_value("HeaderA", "other")).to(be_true());
        expect!(request.has_header_value("HeaderA", "other2")).to(be_false());
    }
}
