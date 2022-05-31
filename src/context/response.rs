use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use crate::headers::HeaderValue;

/// Response that is generated as a result of the webmachine execution
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// status code to return
    pub status: u16,
    /// headers to return
    pub headers: BTreeMap<String, Vec<HeaderValue>>,
    /// Response Body
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Creates a default response (200 OK)
    pub fn default() -> Response {
        Response {
            status: 200,
            headers: BTreeMap::new(),
            body: None,
        }
    }

    /// If the response has the provided header
    pub fn has_header(&self, header: &str) -> bool {
        self.headers
            .keys()
            .find(|k| k.to_uppercase() == header.to_uppercase())
            .is_some()
    }

    /// Adds the header values to the headers
    pub fn add_header(&mut self, header: &str, values: Vec<HeaderValue>) {
        self.headers.insert(header.to_string(), values);
    }

    /// Adds the headers from a HashMap to the headers
    pub fn add_headers(&mut self, headers: HashMap<String, Vec<String>>) {
        for (k, v) in headers {
            self.headers
                .insert(k, v.iter().map(HeaderValue::basic).collect());
        }
    }

    /// Adds standard CORS headers to the response
    pub fn add_cors_headers(&mut self, allowed_methods: &Vec<&str>) {
        let cors_headers = Response::cors_headers(allowed_methods);
        for (k, v) in cors_headers {
            self.add_header(k.as_str(), v.iter().map(HeaderValue::basic).collect());
        }
    }

    /// Returns a HashMap of standard CORS headers
    pub fn cors_headers(allowed_methods: &Vec<&str>) -> HashMap<String, Vec<String>> {
        hashmap! {
          "Access-Control-Allow-Origin".to_string() => vec!["*".to_string()],
          "Access-Control-Allow-Methods".to_string() => allowed_methods.iter().cloned().map_into().collect(),
          "Access-Control-Allow-Headers".to_string() => vec!["Content-Type".to_string()]
        }
    }

    /// If the response has a body
    pub fn has_body(&self) -> bool {
        match &self.body {
            &None => false,
            &Some(ref body) => !body.is_empty(),
        }
    }
}
