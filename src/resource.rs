use chrono::{DateTime, FixedOffset};
use futures::Future;
use std::{collections::HashMap, pin::Pin};

use super::{callback, Callback, Context, Response};

/// Struct to represent a resource in webmachine
#[derive(Clone)]
pub struct Resource<'a> {
    /// This is called just before the final response is constructed and sent. It allows the resource
    /// an opportunity to modify the response after the webmachine has executed.
    pub finalise_response: Option<Callback<'a, ()>>,
    /// This is invoked to render the response for the resource
    pub render_response: Callback<'a, Option<String>>,
    /// Is the resource available? Returning false will result in a '503 Service Not Available'
    /// response. Defaults to true. If the resource is only temporarily not available,
    /// add a 'Retry-After' response header.
    pub available: Callback<'a, bool>,
    /// HTTP methods that are known to the resource. Default includes all standard HTTP methods.
    /// One could override this to allow additional methods
    pub known_methods: Vec<&'a str>,
    /// If the URI is too long to be processed, this should return true, which will result in a
    /// '414 Request URI Too Long' response. Defaults to false.
    pub uri_too_long: Callback<'a, bool>,
    /// HTTP methods that are allowed on this resource. Defaults to GET','HEAD and 'OPTIONS'.
    pub allowed_methods: Vec<&'a str>,
    /// If the request is malformed, this should return true, which will result in a
    /// '400 Malformed Request' response. Defaults to false.
    pub malformed_request: Callback<'a, bool>,
    /// Is the client or request not authorized? Returning a Some<String>
    /// will result in a '401 Unauthorized' response.  Defaults to None. If a Some(String) is
    /// returned, the string will be used as the value in the WWW-Authenticate header.
    pub not_authorized: Callback<'a, Option<String>>,
    /// Is the request or client forbidden? Returning true will result in a '403 Forbidden' response.
    /// Defaults to false.
    pub forbidden: Callback<'a, bool>,
    /// If the request includes any invalid Content-* headers, this should return true, which will
    /// result in a '501 Not Implemented' response. Defaults to false.
    pub unsupported_content_headers: Callback<'a, bool>,
    /// The list of acceptable content types. Defaults to 'application/json'. If the content type
    /// of the request is not in this list, a '415 Unsupported Media Type' response is returned.
    pub acceptable_content_types: Vec<&'a str>,
    /// If the entity length on PUT or POST is invalid, this should return false, which will result
    /// in a '413 Request Entity Too Large' response. Defaults to true.
    pub valid_entity_length: Callback<'a, bool>,
    /// This is called just before the final response is constructed and sent. This allows the
    /// response to be modified. The default implementation adds CORS headers to the response
    pub finish_request: Callback<'a, ()>,
    /// If the OPTIONS method is supported and is used, this returns a HashMap of headers that
    /// should appear in the response. Defaults to CORS headers.
    pub options: Callback<'a, Option<HashMap<String, Vec<String>>>>,
    /// The list of content types that this resource produces. Defaults to 'application/json'. If
    /// more than one is provided, and the client does not supply an Accept header, the first one
    /// will be selected.
    pub produces: Vec<&'a str>,
    /// The list of content languages that this resource provides. Defaults to an empty list,
    /// which represents all languages. If more than one is provided, and the client does not
    /// supply an Accept-Language header, the first one will be selected.
    pub languages_provided: Vec<&'a str>,
    /// The list of charsets that this resource provides. Defaults to an empty list,
    /// which represents all charsets with ISO-8859-1 as the default. If more than one is provided,
    /// and the client does not supply an Accept-Charset header, the first one will be selected.
    pub charsets_provided: Vec<&'a str>,
    /// The list of encodings your resource wants to provide. The encoding will be applied to the
    /// response body automatically by Webmachine. Default includes only the 'identity' encoding.
    pub encodings_provided: Vec<&'a str>,
    /// The list of header names that should be included in the response's Vary header. The standard
    /// content negotiation headers (Accept, Accept-Encoding, Accept-Charset, Accept-Language) do
    /// not need to be specified here as Webmachine will add the correct elements of those
    /// automatically depending on resource behavior. Default is an empty list.
    pub variances: Vec<&'a str>,
    /// Does the resource exist? Returning a false value will result in a '404 Not Found' response
    /// unless it is a PUT or POST. Defaults to true.
    pub resource_exists: Callback<'a, bool>,
    /// If this resource is known to have existed previously, this should return true. Default is false.
    pub previously_existed: Callback<'a, bool>,
    /// If this resource has moved to a new location permanently, this should return the new
    /// location as a String. Default is to return None
    pub moved_permanently: Callback<'a, Option<String>>,
    /// If this resource has moved to a new location temporarily, this should return the new
    /// location as a String. Default is to return None
    pub moved_temporarily: Callback<'a, Option<String>>,
    /// If this returns true, the client will receive a '409 Conflict' response. This is only
    /// called for PUT requests. Default is false.
    pub is_conflict: Callback<'a, bool>,
    /// Return true if the resource accepts POST requests to nonexistent resources. Defaults to false.
    pub allow_missing_post: Callback<'a, bool>,
    /// If this returns a value, it will be used as the value of the ETag header and for
    /// comparison in conditional requests. Default is None.
    pub generate_etag: Callback<'a, Option<String>>,
    /// Returns the last modified date and time of the resource which will be added as the
    /// Last-Modified header in the response and used in negotiating conditional requests.
    /// Default is None
    pub last_modified: Callback<'a, Option<DateTime<FixedOffset>>>,
    /// Called when a DELETE request should be enacted. Return `Ok(true)` if the deletion succeeded,
    /// and `Ok(false)` if the deletion was accepted but cannot yet be guaranteed to have finished.
    /// If the delete fails for any reason, return an Err with the status code you wish returned
    /// (a 500 status makes sense).
    /// Defaults to `Ok(true)`.
    pub delete_resource: Callback<'a, Result<bool, u16>>,
    /// If POST requests should be treated as a request to put content into a (potentially new)
    /// resource as opposed to a generic submission for processing, then this should return true.
    /// If it does return true, then `create_path` will be called and the rest of the request will
    /// be treated much like a PUT to the path returned by that call. Default is false.
    pub post_is_create: Callback<'a, bool>,
    /// If `post_is_create` returns false, then this will be called to process any POST request.
    /// If it succeeds, return `Ok(true)`, `Ok(false)` otherwise. If it fails for any reason,
    /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
    /// Default is false. If you want the result of processing the POST to be a redirect, set
    /// `context.redirect` to true.
    pub process_post: Callback<'a, Result<bool, u16>>,
    /// This will be called on a POST request if `post_is_create` returns true. It should create
    /// the new resource and return the path as a valid URI part following the dispatcher prefix.
    /// That path will replace the previous one in the return value of `WebmachineRequest.request_path`
    /// for all subsequent resource function calls in the course of this request and will be set
    /// as the value of the Location header of the response. If it fails for any reason,
    /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
    /// Default will return an `Ok(WebmachineRequest.request_path)`. If you want the result of
    /// processing the POST to be a redirect, set `context.redirect` to true.
    pub create_path: Callback<'a, Result<String, u16>>,
    /// This will be called to process any PUT request. If it succeeds, return `Ok(true)`,
    /// `Ok(false)` otherwise. If it fails for any reason, return an Err with the status code
    /// you wish returned (e.g., a 500 status makes sense). Default is `Ok(true)`
    pub process_put: Callback<'a, Result<bool, u16>>,
    /// If this returns true, then it is assumed that multiple representations of the response are
    /// possible and a single one cannot be automatically chosen, so a 300 Multiple Choices will
    /// be sent instead of a 200. Default is false.
    pub multiple_choices: Callback<'a, bool>,
    /// If the resource expires, this should return the date/time it expires. Default is None.
    pub expires: Callback<'a, Option<DateTime<FixedOffset>>>,
}

fn true_fn(
    _: &mut Context,
    _: &Resource,
) -> Pin<Box<dyn Future<Output = bool> + Send>> {
    Box::pin(async { true })
}

fn false_fn(
    _: &mut Context,
    _: &Resource,
) -> Pin<Box<dyn Future<Output = bool> + Send>> {
    Box::pin(async { false })
}

fn none_fn<T>(
    _: &mut Context,
    _: &Resource,
) -> Pin<Box<dyn Future<Output = Option<T>> + Send>> {
    Box::pin(async { None })
}

impl<'a> Default for Resource<'a> {
    fn default() -> Resource<'a> {
        Resource {
            finalise_response: None,
            available: callback(&true_fn),
            known_methods: vec![
                "OPTIONS", "GET", "POST", "PUT", "DELETE", "HEAD", "TRACE", "CONNECT", "PATCH",
            ],
            uri_too_long: callback(&false_fn),
            allowed_methods: vec!["OPTIONS", "GET", "HEAD"],
            malformed_request: callback(&false_fn),
            not_authorized: callback(&none_fn),
            forbidden: callback(&false_fn),
            unsupported_content_headers: callback(&false_fn),
            acceptable_content_types: vec!["application/json"],
            valid_entity_length: callback(&true_fn),
            finish_request: callback(&|context, resource| {
                context.response.add_cors_headers(&resource.allowed_methods);
                Box::pin(async {})
            }),
            options: callback(&|_, resource| {
                let res = Response::cors_headers(&resource.allowed_methods);
                Box::pin(async {
                    Some(res)
                })
            }),
            produces: vec!["application/json"],
            languages_provided: Vec::new(),
            charsets_provided: Vec::new(),
            encodings_provided: vec!["identity"],
            variances: Vec::new(),
            resource_exists: callback(&true_fn),
            previously_existed: callback(&false_fn),
            moved_permanently: callback(&none_fn),
            moved_temporarily: callback(&none_fn),
            is_conflict: callback(&false_fn),
            allow_missing_post: callback(&false_fn),
            generate_etag: callback(&none_fn),
            last_modified: callback(&none_fn),
            delete_resource: callback(&|_, _| Box::pin(async { Ok(true) })),
            post_is_create: callback(&false_fn),
            process_post: callback(&|_, _| Box::pin(async { Ok(false) })),
            process_put: callback(&|_, _| Box::pin(async { Ok(true) })),
            multiple_choices: callback(&false_fn),
            create_path: callback(&|context, _| {
                let path = context.request.request_path.clone();
                Box::pin(async { Ok(path) })
            }),
            expires: callback(&none_fn),
            render_response: callback(&none_fn),
        }
    }
}
