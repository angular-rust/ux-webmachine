#![doc(html_logo_url = "https://dudochkin-victor.github.io/assets/ruex/logo.svg")]

#![warn(missing_docs)]

//! # webmachine-rust
//!
//! Port of Webmachine-Ruby [https://github.com/webmachine/webmachine-ruby][1] to Rust.
//! 
//! webmachine-rust is a port of the Ruby version of webmachine. It implements a finite state machine for the HTTP protocol
//! that provides semantic HTTP handling (based on the [diagram from the webmachine project](https://webmachine.github.io/images/http-headers-status-v3.png)).
//! It is basically a HTTP toolkit for building HTTP-friendly applications using the [Hyper] rust crate.
//! 
//! Webmachine-rust works with Hyper and sits between the Hyper Handler and your application code. It provides a resource struct
//! with callbacks to handle the decisions required as the state machine is executed against the request with the following sequence.
//! 
//! REQUEST -> Hyper Handler -> WebmachineDispatcher -> WebmachineResource -> Your application code -> WebmachineResponse -> Hyper -> RESPONSE
//! 
//! ## Features
//! 
//! - Handles the hard parts of content negotiation, conditional requests, and response codes for you.
//! - Provides a resource struct with points of extension to let you describe what is relevant about your particular resource.
//! 
//! ## Missing Features
//! 
//! Currently, the following features from webmachine-ruby have not been implemented:
//! 
//! - Visual debugger
//! - Streaming response bodies
//! 
//! ## Implementation Deficiencies:
//! 
//! This implementation has the following deficiencies:
//! 
//! - Automatically decoding request bodies and encoding response bodies.
//! - No easy mechanism to generate bodies with different content types (e.g. JSON vs. XML).
//! - No easy mechanism for handling sub-paths in a resource.
//! - Dynamically determining the methods allowed on the resource.
//! 
//! ## Getting started with Hyper
//! 
//! Follow the getting started documentation from the Hyper crate to setup a Hyper service for your server.
//! You need to define a WebmachineDispatcher that maps resource paths to your webmachine resources (WebmachineResource).
//! Each WebmachineResource defines all the callbacks (via Closures) and values required to implement a resource.
//! The WebmachineDispatcher implementes the Hyper Service trait, so you can pass it to the `make_service_fn`.
//! 
//! Note: This example uses the maplit crate to provide the `btreemap` macro and the log crate for the logging macros.
//! 
//!  ```no_run
//!  # #[macro_use] extern crate log;
//!  # #[macro_use] extern crate maplit;
//! 
//!  use webmachine::*;
//!  use webmachine::{context::*, headers::*};
//!  use serde_json::{Value, json};
//!  use hyper::{server::Server, service::make_service_fn};
//!  use std::{io::Read, net::SocketAddr, convert::Infallible};
//! 
//!  # fn main() {}
//!  // setup the dispatcher, which maps paths to resources. The requirement of make_service_fn is
//!  // that it has a static lifetime
//!  fn dispatcher() -> Dispatcher<'static> {
//!    Dispatcher {
//!        routes: btreemap!{
//!           "/myresource" => Resource {
//!             // Methods allowed on this resource
//!             allowed_methods: vec!["OPTIONS", "GET", "HEAD", "POST"],
//!             // if the resource exists callback
//!             resource_exists: callback(&|_, _| Box::pin(async { true })),
//!             // callback to render the response for the resource
//!             render_response: callback(&|_, _| Box::pin(async {
//!                 let json_response = json!({
//!                    "data": [1, 2, 3, 4]
//!                 });
//!                 Some(json_response.to_string())
//!             })),
//!             // callback to process the post for the resource
//!             process_post: callback(&|_, _|  Box::pin(async {
//!                 // Handle the post here
//!                 Ok(true)
//!             })),
//!             // default everything else
//!             .. Resource::default()
//!           }
//!       }
//!    }
//!  }
//! 
//!  async fn start_server() -> Result<(), String> {
//!    // Create a Hyper server that delegates to the dispatcher
//!    let addr = "0.0.0.0:8080".parse().unwrap();
//!    let make_svc = make_service_fn(|_| async { Ok::<_, Infallible>(dispatcher()) });
//!    match Server::try_bind(&addr) {
//!      Ok(server) => {
//!        // start the actual server
//!        server.serve(make_svc).await;
//!      },
//!      Err(err) => {
//!        error!("could not start server: {}", err);
//!      }
//!    };
//!    Ok(())
//!  }
//!  ```
//! 
//! ## Example implementations
//! 
//! For an example of a project using this crate, have a look at the [Pact Mock Server](https://github.com/pact-foundation/pact-reference/tree/master/rust/v1/pact_mock_server_cli) from the Pact reference implementation.
//! 
//! [1]: https://github.com/webmachine/webmachine-ruby
//! [Hyper]: https://crates.io/crates/hyper


#![warn(missing_docs)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate maplit;

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, FixedOffset, Utc};
use context::{Context, Request, Response};
use futures::{lock::Mutex, TryStreamExt};
use headers::HeaderValue;
use http::request::Parts;
use hyper::service::Service;
use itertools::Itertools;
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::Poll,
};

pub mod cache;

mod dispatcher;
pub use self::dispatcher::*;

mod enums;
use self::enums::*;

#[macro_use]
pub mod headers;

pub mod content_negotiation;
pub mod context;

mod resource;
pub use self::resource::*;

pub mod wamp {
    //! Wamp(v2) support
    pub use wampire::*;
}

// /// Type of a Webmachine resource callback
// pub type WebmachineCallback<'a, T> =
//     Arc<Mutex<Box<dyn Fn(&mut WebmachineContext, &WebmachineResource) -> T + Send + Sync + 'a>>>;

/// Type of a Webmachine resource callback
pub type Callback<'a, T> = Arc<
    Mutex<
        Box<
            dyn Fn(&mut Context, &Resource) -> Pin<Box<dyn Future<Output = T> + Send>>
                + Send
                + Sync
                + 'a,
        >,
    >,
>;

/// Wrap a callback in a structure that is safe to call between threads
pub fn callback<T, RT>(cb: &T) -> Callback<RT>
where
    T: Fn(&mut Context, &Resource) -> Pin<Box<dyn Future<Output = RT> + Send>> + Send + Sync,
{
    Arc::new(Mutex::new(Box::new(cb)))
}

fn sanitise_path(path: &str) -> Vec<String> {
    path.split("/")
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

fn join_paths(base: &Vec<String>, path: &Vec<String>) -> String {
    let mut paths = base.clone();
    paths.extend_from_slice(path);
    let filtered: Vec<String> = paths.iter().cloned().filter(|p| !p.is_empty()).collect();
    if filtered.is_empty() {
        "/".to_string()
    } else {
        let new_path = filtered.iter().join("/");
        if new_path.starts_with("/") {
            new_path
        } else {
            "/".to_owned() + &new_path
        }
    }
}

lazy_static! {
    static ref TRANSITION_MAP: HashMap<Decision, Transition> = hashmap! {
        Decision::Start => Transition::To(Decision::B13Available),
        Decision::B3Options => Transition::Branch(Decision::A3Options, Decision::C3AcceptExists),
        Decision::B4RequestEntityTooLarge => Transition::Branch(Decision::End(413), Decision::B3Options),
        Decision::B5UnknownContentType => Transition::Branch(Decision::End(415), Decision::B4RequestEntityTooLarge),
        Decision::B6UnsupportedContentHeader => Transition::Branch(Decision::End(501), Decision::B5UnknownContentType),
        Decision::B7Forbidden => Transition::Branch(Decision::End(403), Decision::B6UnsupportedContentHeader),
        Decision::B8Authorized => Transition::Branch(Decision::B7Forbidden, Decision::End(401)),
        Decision::B9MalformedRequest => Transition::Branch(Decision::End(400), Decision::B8Authorized),
        Decision::B10MethodAllowed => Transition::Branch(Decision::B9MalformedRequest, Decision::End(405)),
        Decision::B11UriTooLong => Transition::Branch(Decision::End(414), Decision::B10MethodAllowed),
        Decision::B12KnownMethod => Transition::Branch(Decision::B11UriTooLong, Decision::End(501)),
        Decision::B13Available => Transition::Branch(Decision::B12KnownMethod, Decision::End(503)),
        Decision::C3AcceptExists => Transition::Branch(Decision::C4AcceptableMediaTypeAvailable, Decision::D4AcceptLanguageExists),
        Decision::C4AcceptableMediaTypeAvailable => Transition::Branch(Decision::D4AcceptLanguageExists, Decision::End(406)),
        Decision::D4AcceptLanguageExists => Transition::Branch(Decision::D5AcceptableLanguageAvailable, Decision::E5AcceptCharsetExists),
        Decision::D5AcceptableLanguageAvailable => Transition::Branch(Decision::E5AcceptCharsetExists, Decision::End(406)),
        Decision::E5AcceptCharsetExists => Transition::Branch(Decision::E6AcceptableCharsetAvailable, Decision::F6AcceptEncodingExists),
        Decision::E6AcceptableCharsetAvailable => Transition::Branch(Decision::F6AcceptEncodingExists, Decision::End(406)),
        Decision::F6AcceptEncodingExists => Transition::Branch(Decision::F7AcceptableEncodingAvailable, Decision::G7ResourceExists),
        Decision::F7AcceptableEncodingAvailable => Transition::Branch(Decision::G7ResourceExists, Decision::End(406)),
        Decision::G7ResourceExists => Transition::Branch(Decision::G8IfMatchExists, Decision::H7IfMatchStarExists),
        Decision::G8IfMatchExists => Transition::Branch(Decision::G9IfMatchStarExists, Decision::H10IfUnmodifiedSinceExists),
        Decision::G9IfMatchStarExists => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::G11EtagInIfMatch),
        Decision::G11EtagInIfMatch => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::End(412)),
        Decision::H7IfMatchStarExists => Transition::Branch(Decision::End(412), Decision::I7Put),
        Decision::H10IfUnmodifiedSinceExists => Transition::Branch(Decision::H11IfUnmodifiedSinceValid, Decision::I12IfNoneMatchExists),
        Decision::H11IfUnmodifiedSinceValid => Transition::Branch(Decision::H12LastModifiedGreaterThanUMS, Decision::I12IfNoneMatchExists),
        Decision::H12LastModifiedGreaterThanUMS => Transition::Branch(Decision::End(412), Decision::I12IfNoneMatchExists),
        Decision::I4HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::P3Conflict),
        Decision::I7Put => Transition::Branch(Decision::I4HasMovedPermanently, Decision::K7ResourcePreviouslyExisted),
        Decision::I12IfNoneMatchExists => Transition::Branch(Decision::I13IfNoneMatchStarExists, Decision::L13IfModifiedSinceExists),
        Decision::I13IfNoneMatchStarExists => Transition::Branch(Decision::J18GetHead, Decision::K13ETagInIfNoneMatch),
        Decision::J18GetHead => Transition::Branch(Decision::End(304), Decision::End(412)),
        Decision::K13ETagInIfNoneMatch => Transition::Branch(Decision::J18GetHead, Decision::L13IfModifiedSinceExists),
        Decision::K5HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::L5HasMovedTemporarily),
        Decision::K7ResourcePreviouslyExisted => Transition::Branch(Decision::K5HasMovedPermanently, Decision::L7Post),
        Decision::L5HasMovedTemporarily => Transition::Branch(Decision::End(307), Decision::M5Post),
        Decision::L7Post => Transition::Branch(Decision::M7PostToMissingResource, Decision::End(404)),
        Decision::L13IfModifiedSinceExists => Transition::Branch(Decision::L14IfModifiedSinceValid, Decision::M16Delete),
        Decision::L14IfModifiedSinceValid => Transition::Branch(Decision::L15IfModifiedSinceGreaterThanNow, Decision::M16Delete),
        Decision::L15IfModifiedSinceGreaterThanNow => Transition::Branch(Decision::M16Delete, Decision::L17IfLastModifiedGreaterThanMS),
        Decision::L17IfLastModifiedGreaterThanMS => Transition::Branch(Decision::M16Delete, Decision::End(304)),
        Decision::M5Post => Transition::Branch(Decision::N5PostToMissingResource, Decision::End(410)),
        Decision::M7PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(404)),
        Decision::M16Delete => Transition::Branch(Decision::M20DeleteEnacted, Decision::N16Post),
        Decision::M20DeleteEnacted => Transition::Branch(Decision::O20ResponseHasBody, Decision::End(202)),
        Decision::N5PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(410)),
        Decision::N11Redirect => Transition::Branch(Decision::End(303), Decision::P11NewResource),
        Decision::N16Post => Transition::Branch(Decision::N11Redirect, Decision::O16Put),
        Decision::O14Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::O16Put => Transition::Branch(Decision::O14Conflict, Decision::O18MultipleRepresentations),
        Decision::P3Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::P11NewResource => Transition::Branch(Decision::End(201), Decision::O20ResponseHasBody),
        Decision::O18MultipleRepresentations => Transition::Branch(Decision::End(300), Decision::End(200)),
        Decision::O20ResponseHasBody => Transition::Branch(Decision::O18MultipleRepresentations, Decision::End(204))
    };
}

async fn resource_etag_matches_header_values(
    resource: &Resource<'_>,
    context: &mut Context,
    header: &str,
) -> bool {
    let header_values = context.request.find_header(header);
    let callback = resource.generate_etag.lock().await;

    match callback.deref()(context, resource).await {
        Some(etag) => header_values
            .iter()
            .find(|val| {
                if val.value.starts_with("W/") {
                    val.weak_etag().unwrap() == etag
                } else {
                    val.value == etag
                }
            })
            .is_some(),
        None => false,
    }
}

fn validate_header_date(
    request: &Request,
    header: &str,
    context_meta: &mut Option<DateTime<FixedOffset>>,
) -> bool {
    let header_values = request.find_header(header);
    if let Some(date_value) = header_values.first() {
        match DateTime::parse_from_rfc2822(&date_value.value) {
            Ok(datetime) => {
                *context_meta = Some(datetime.clone());
                true
            }
            Err(err) => {
                debug!(
                    "Failed to parse '{}' header value '{:?}' - {}",
                    header, date_value, err
                );
                false
            }
        }
    } else {
        false
    }
}

async fn execute_decision(
    decision: &Decision,
    context: &mut Context,
    resource: &Resource<'_>,
) -> DecisionResult {
    match decision {
        Decision::B10MethodAllowed => {
            match resource
                .allowed_methods
                .iter()
                .find(|m| m.to_uppercase() == context.request.method.to_uppercase())
            {
                Some(_) => {
                    DecisionResult::True("method is in the list of allowed methods".to_string())
                }
                None => {
                    context.response.add_header(
                        "Allow",
                        resource
                            .allowed_methods
                            .iter()
                            .cloned()
                            .map(HeaderValue::basic)
                            .collect(),
                    );
                    DecisionResult::False(
                        "method is not in the list of allowed methods".to_string(),
                    )
                }
            }
        }
        Decision::B11UriTooLong => {
            let callback = resource.uri_too_long.lock().await;
            DecisionResult::wrap(callback.deref()(context, resource).await, "URI too long")
        }
        Decision::B12KnownMethod => DecisionResult::wrap(
            resource
                .known_methods
                .iter()
                .find(|m| m.to_uppercase() == context.request.method.to_uppercase())
                .is_some(),
            "known method",
        ),
        Decision::B13Available => {
            let callback = resource.available.lock().await;
            DecisionResult::wrap(callback.deref()(context, resource).await, "available")
        }
        Decision::B9MalformedRequest => {
            let callback = resource.malformed_request.lock().await;
            DecisionResult::wrap(
                callback.deref()(context, resource).await,
                "malformed request",
            )
        }
        Decision::B8Authorized => {
            let callback = resource.not_authorized.lock().await;
            match callback.deref()(context, resource).await {
                Some(realm) => {
                    context.response.add_header(
                        "WWW-Authenticate",
                        vec![HeaderValue::parse_string(realm.as_str())],
                    );
                    DecisionResult::False("is not authorized".to_string())
                }
                None => DecisionResult::True("is not authorized".to_string()),
            }
        }
        Decision::B7Forbidden => {
            let callback = resource.forbidden.lock().await;
            DecisionResult::wrap(callback.deref()(context, resource).await, "forbidden")
        }
        Decision::B6UnsupportedContentHeader => {
            let callback = resource.unsupported_content_headers.lock().await;
            DecisionResult::wrap(
                callback.deref()(context, resource).await,
                "unsupported content headers",
            )
        }
        Decision::B5UnknownContentType => DecisionResult::wrap(
            context.request.is_put_or_post()
                && resource
                    .acceptable_content_types
                    .iter()
                    .find(|ct| context.request.content_type().to_uppercase() == ct.to_uppercase())
                    .is_none(),
            "acceptable content types",
        ),
        Decision::B4RequestEntityTooLarge => {
            let callback = resource.valid_entity_length.lock().await;
            DecisionResult::wrap(
                context.request.is_put_or_post() && !callback.deref()(context, resource).await,
                "valid entity length",
            )
        }
        Decision::B3Options => DecisionResult::wrap(context.request.is_options(), "options"),
        Decision::C3AcceptExists => {
            DecisionResult::wrap(context.request.has_accept_header(), "has accept header")
        }
        Decision::C4AcceptableMediaTypeAvailable => {
            match content_negotiation::matching_content_type(resource, &context.request) {
                Some(media_type) => {
                    context.selected_media_type = Some(media_type);
                    DecisionResult::True("acceptable media type is available".to_string())
                }
                None => DecisionResult::False("acceptable media type is not available".to_string()),
            }
        }
        Decision::D4AcceptLanguageExists => DecisionResult::wrap(
            context.request.has_accept_language_header(),
            "has accept language header",
        ),
        Decision::D5AcceptableLanguageAvailable => {
            match content_negotiation::matching_language(resource, &context.request) {
                Some(language) => {
                    if language != "*" {
                        context.selected_language = Some(language.clone());
                        context.response.add_header(
                            "Content-Language",
                            vec![HeaderValue::parse_string(&language)],
                        );
                    }
                    DecisionResult::True("acceptable language is available".to_string())
                }
                None => DecisionResult::False("acceptable language is not available".to_string()),
            }
        }
        Decision::E5AcceptCharsetExists => DecisionResult::wrap(
            context.request.has_accept_charset_header(),
            "accept charset exists",
        ),
        Decision::E6AcceptableCharsetAvailable => {
            match content_negotiation::matching_charset(resource, &context.request) {
                Some(charset) => {
                    if charset != "*" {
                        context.selected_charset = Some(charset.clone());
                    }
                    DecisionResult::True("acceptable charset is available".to_string())
                }
                None => DecisionResult::False("acceptable charset is not available".to_string()),
            }
        }
        Decision::F6AcceptEncodingExists => DecisionResult::wrap(
            context.request.has_accept_encoding_header(),
            "accept encoding exists",
        ),
        Decision::F7AcceptableEncodingAvailable => {
            match content_negotiation::matching_encoding(resource, &context.request) {
                Some(encoding) => {
                    context.selected_encoding = Some(encoding.clone());
                    if encoding != "identity" {
                        context.response.add_header(
                            "Content-Encoding",
                            vec![HeaderValue::parse_string(&encoding)],
                        );
                    }
                    DecisionResult::True("acceptable encoding is available".to_string())
                }
                None => DecisionResult::False("acceptable encoding is not available".to_string()),
            }
        }
        Decision::G7ResourceExists => {
            let callback = resource.resource_exists.lock().await;
            DecisionResult::wrap(callback.deref()(context, resource).await, "resource exists")
        }
        Decision::G8IfMatchExists => {
            DecisionResult::wrap(context.request.has_header("If-Match"), "match exists")
        }
        Decision::G9IfMatchStarExists | &Decision::H7IfMatchStarExists => DecisionResult::wrap(
            context.request.has_header_value("If-Match", "*"),
            "match star exists",
        ),
        Decision::G11EtagInIfMatch => DecisionResult::wrap(
            resource_etag_matches_header_values(resource, context, "If-Match").await,
            "etag in if match",
        ),
        Decision::H10IfUnmodifiedSinceExists => DecisionResult::wrap(
            context.request.has_header("If-Unmodified-Since"),
            "unmodified since exists",
        ),
        Decision::H11IfUnmodifiedSinceValid => DecisionResult::wrap(
            validate_header_date(
                &context.request,
                "If-Unmodified-Since",
                &mut context.if_unmodified_since,
            ),
            "unmodified since valid",
        ),
        Decision::H12LastModifiedGreaterThanUMS => match context.if_unmodified_since {
            Some(unmodified_since) => {
                let callback = resource.last_modified.lock().await;
                match callback.deref()(context, resource).await {
                    Some(datetime) => DecisionResult::wrap(
                        datetime > unmodified_since,
                        "resource last modified date is greater than unmodified since",
                    ),
                    None => DecisionResult::False("resource has no last modified date".to_string()),
                }
            }
            None => {
                DecisionResult::False("resource does not provide last modified date".to_string())
            }
        },
        Decision::I7Put => {
            if context.request.is_put() {
                context.new_resource = true;
                DecisionResult::True("is a PUT request".to_string())
            } else {
                DecisionResult::False("is not a PUT request".to_string())
            }
        }
        Decision::I12IfNoneMatchExists => DecisionResult::wrap(
            context.request.has_header("If-None-Match"),
            "none match exists",
        ),
        Decision::I13IfNoneMatchStarExists => DecisionResult::wrap(
            context.request.has_header_value("If-None-Match", "*"),
            "none match star exists",
        ),
        Decision::J18GetHead => {
            DecisionResult::wrap(context.request.is_get_or_head(), "is GET or HEAD request")
        }
        Decision::K7ResourcePreviouslyExisted => {
            let callback = resource.previously_existed.lock().await;
            DecisionResult::wrap(
                callback.deref()(context, resource).await,
                "resource previously existed",
            )
        }
        Decision::K13ETagInIfNoneMatch => DecisionResult::wrap(
            resource_etag_matches_header_values(resource, context, "If-None-Match").await,
            "ETag in if none match",
        ),
        Decision::L5HasMovedTemporarily => {
            let callback = resource.moved_temporarily.lock().await;
            match callback.deref()(context, resource).await {
                Some(location) => {
                    context
                        .response
                        .add_header("Location", vec![HeaderValue::basic(&location)]);
                    DecisionResult::True("resource has moved temporarily".to_string())
                }
                None => DecisionResult::False("resource has not moved temporarily".to_string()),
            }
        }
        Decision::L7Post | &Decision::M5Post | &Decision::N16Post => {
            DecisionResult::wrap(context.request.is_post(), "a POST request")
        }
        Decision::L13IfModifiedSinceExists => DecisionResult::wrap(
            context.request.has_header("If-Modified-Since"),
            "if modified since exists",
        ),
        Decision::L14IfModifiedSinceValid => DecisionResult::wrap(
            validate_header_date(
                &context.request,
                "If-Modified-Since",
                &mut context.if_modified_since,
            ),
            "modified since valid",
        ),
        Decision::L15IfModifiedSinceGreaterThanNow => {
            let datetime = context.if_modified_since.unwrap();
            let timezone = datetime.timezone();
            DecisionResult::wrap(
                datetime > Utc::now().with_timezone(&timezone),
                "modified since greater than now",
            )
        }
        Decision::L17IfLastModifiedGreaterThanMS => match context.if_modified_since {
            Some(unmodified_since) => {
                let callback = resource.last_modified.lock().await;
                match callback.deref()(context, resource).await {
                    Some(datetime) => DecisionResult::wrap(
                        datetime > unmodified_since,
                        "last modified greater than modified since",
                    ),
                    None => DecisionResult::False("resource has no last modified date".to_string()),
                }
            }
            None => DecisionResult::False("resource does not return if_modified_since".to_string()),
        },
        Decision::I4HasMovedPermanently | &Decision::K5HasMovedPermanently => {
            let callback = resource.moved_permanently.lock().await;
            match callback.deref()(context, resource).await {
                Some(location) => {
                    context
                        .response
                        .add_header("Location", vec![HeaderValue::basic(&location)]);
                    DecisionResult::True("resource has moved permanently".to_string())
                }
                None => DecisionResult::False("resource has not moved permanently".to_string()),
            }
        }
        Decision::M7PostToMissingResource | &Decision::N5PostToMissingResource => {
            let callback = resource.allow_missing_post.lock().await;
            if callback.deref()(context, resource).await {
                context.new_resource = true;
                DecisionResult::True("resource allows POST to missing resource".to_string())
            } else {
                DecisionResult::False(
                    "resource does not allow POST to missing resource".to_string(),
                )
            }
        }
        Decision::M16Delete => {
            DecisionResult::wrap(context.request.is_delete(), "a DELETE request")
        }
        Decision::M20DeleteEnacted => {
            let callback = resource.delete_resource.lock().await;
            match callback.deref()(context, resource).await {
                Ok(result) => DecisionResult::wrap(result, "resource DELETE succeeded"),
                Err(status) => DecisionResult::StatusCode(status),
            }
        }
        Decision::N11Redirect => {
            let callback = resource.post_is_create.lock().await;
            if callback.deref()(context, resource).await {
                let callback = resource.create_path.lock().await;
                match callback.deref()(context, resource).await {
                    Ok(path) => {
                        let base_path = sanitise_path(&context.request.base_path);
                        let new_path = join_paths(&base_path, &sanitise_path(&path));
                        context.request.request_path = path.clone();
                        context
                            .response
                            .add_header("Location", vec![HeaderValue::basic(&new_path)]);
                        DecisionResult::wrap(context.redirect, "should redirect")
                    }
                    Err(status) => DecisionResult::StatusCode(status),
                }
            } else {
                let callback = resource.process_post.lock().await;
                match callback.deref()(context, resource).await {
                    Ok(_) => DecisionResult::wrap(context.redirect, "processing POST succeeded"),
                    Err(status) => DecisionResult::StatusCode(status),
                }
            }
        }
        Decision::P3Conflict | &Decision::O14Conflict => {
            let callback = resource.is_conflict.lock().await;
            DecisionResult::wrap(
                callback.deref()(context, resource).await,
                "resource conflict",
            )
        }
        Decision::P11NewResource => {
            if context.request.is_put() {
                let callback = resource.process_put.lock().await;
                match callback.deref()(context, resource).await {
                    Ok(_) => DecisionResult::wrap(context.new_resource, "process PUT succeeded"),
                    Err(status) => DecisionResult::StatusCode(status),
                }
            } else {
                DecisionResult::wrap(context.new_resource, "new resource creation succeeded")
            }
        }
        Decision::O16Put => DecisionResult::wrap(context.request.is_put(), "a PUT request"),
        Decision::O18MultipleRepresentations => {
            let callback = resource.multiple_choices.lock().await;
            DecisionResult::wrap(
                callback.deref()(context, resource).await,
                "multiple choices exist",
            )
        }
        Decision::O20ResponseHasBody => {
            DecisionResult::wrap(context.response.has_body(), "response has a body")
        }
        _ => DecisionResult::False("default decision is false".to_string()),
    }
}

async fn execute_state_machine(context: &mut Context, resource: &Resource<'_>) {
    let mut state = Decision::Start;
    let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
    let mut loop_count = 0;
    while !state.is_terminal() {
        loop_count += 1;
        if loop_count >= MAX_STATE_MACHINE_TRANSITIONS {
            panic!(
                "State machine has not terminated within {} transitions!",
                loop_count
            );
        }
        trace!("state is {:?}", state);
        state = match TRANSITION_MAP.get(&state) {
            Some(transition) => match transition {
                &Transition::To(ref decision) => {
                    trace!("Transitioning to {:?}", decision);
                    decision.clone()
                }
                &Transition::Branch(ref decision_true, ref decision_false) => {
                    match execute_decision(&state, context, resource).await {
                        DecisionResult::True(reason) => {
                            trace!(
                                "Transitioning from {:?} to {:?} as decision is true -> {}",
                                state,
                                decision_true,
                                reason
                            );
                            decisions.push((state, true, decision_true.clone()));
                            decision_true.clone()
                        }
                        DecisionResult::False(reason) => {
                            trace!(
                                "Transitioning from {:?} to {:?} as decision is false -> {}",
                                state,
                                decision_false,
                                reason
                            );
                            decisions.push((state, false, decision_false.clone()));
                            decision_false.clone()
                        }
                        DecisionResult::StatusCode(code) => {
                            let decision = Decision::End(code);
                            trace!(
                                "Transitioning from {:?} to {:?} as decision is a status code",
                                state,
                                decision
                            );
                            decisions.push((state, false, decision.clone()));
                            decision.clone()
                        }
                    }
                }
            },
            None => {
                error!(
                    "Error transitioning from {:?}, the TRANSITION_MAP is mis-configured",
                    state
                );
                decisions.push((state, false, Decision::End(500)));
                Decision::End(500)
            }
        }
    }
    trace!("Final state is {:?}", state);
    match state {
        Decision::End(status) => context.response.status = status,
        Decision::A3Options => {
            context.response.status = 204;
            let callback = resource.options.lock().await;
            match callback.deref()(context, resource).await {
                Some(headers) => context.response.add_headers(headers),
                None => (),
            }
        }
        _ => (),
    }
}

fn update_paths_for_resource(request: &mut Request, base_path: &str) {
    request.base_path = base_path.into();
    if request.request_path.len() > base_path.len() {
        let request_path = request.request_path.clone();
        let subpath = request_path.split_at(base_path.len()).1;
        if subpath.starts_with("/") {
            request.request_path = subpath.to_string();
        } else {
            request.request_path = "/".to_owned() + subpath;
        }
    } else {
        request.request_path = "/".to_string();
    }
}

fn parse_header_values(value: &str) -> Vec<HeaderValue> {
    if value.is_empty() {
        Vec::new()
    } else {
        value
            .split(',')
            .map(|s| HeaderValue::parse_string(s.trim()))
            .collect()
    }
}

fn headers_from_http_request(req: &Parts) -> HashMap<String, Vec<HeaderValue>> {
    req.headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                parse_header_values(value.to_str().unwrap_or_default()),
            )
        })
        .collect()
}

fn decode_query(query: &str) -> String {
    let mut chars = query.chars();
    let mut ch = chars.next();
    let mut result = String::new();

    while ch.is_some() {
        let c = ch.unwrap();
        if c == '%' {
            let c1 = chars.next();
            let c2 = chars.next();
            match (c1, c2) {
                (Some(v1), Some(v2)) => {
                    let mut s = String::new();
                    s.push(v1);
                    s.push(v2);
                    let decoded: Result<Vec<u8>, _> = hex::decode(s);
                    match decoded {
                        Ok(n) => result.push(n[0] as char),
                        Err(_) => {
                            result.push('%');
                            result.push(v1);
                            result.push(v2);
                        }
                    }
                }
                (Some(v1), None) => {
                    result.push('%');
                    result.push(v1);
                }
                _ => result.push('%'),
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }

        ch = chars.next();
    }

    result
}

fn parse_query(query: &str) -> HashMap<String, Vec<String>> {
    if !query.is_empty() {
        query
            .split("&")
            .map(|kv| {
                if kv.is_empty() {
                    vec![]
                } else if kv.contains("=") {
                    kv.splitn(2, "=").collect::<Vec<&str>>()
                } else {
                    vec![kv]
                }
            })
            .fold(HashMap::new(), |mut map, name_value| {
                if !name_value.is_empty() {
                    let name = decode_query(name_value[0]);
                    let value = if name_value.len() > 1 {
                        decode_query(name_value[1])
                    } else {
                        String::new()
                    };
                    map.entry(name).or_insert(vec![]).push(value);
                }
                map
            })
    } else {
        HashMap::new()
    }
}

async fn finalise_response(context: &mut Context, resource: &Resource<'_>) {
    if !context.response.has_header("Content-Type") {
        let media_type = match &context.selected_media_type {
            &Some(ref media_type) => media_type.clone(),
            &None => "application/json".to_string(),
        };
        let charset = match &context.selected_charset {
            &Some(ref charset) => charset.clone(),
            &None => "ISO-8859-1".to_string(),
        };
        let header = HeaderValue {
            value: media_type,
            params: hashmap! { "charset".to_string() => charset },
            quote: false,
        };
        context.response.add_header("Content-Type", vec![header]);
    }

    let mut vary_header = if !context.response.has_header("Vary") {
        resource
            .variances
            .iter()
            .map(|h| HeaderValue::parse_string(h.clone()))
            .collect()
    } else {
        Vec::new()
    };

    if resource.languages_provided.len() > 1 {
        vary_header.push(h!("Accept-Language"));
    }
    if resource.charsets_provided.len() > 1 {
        vary_header.push(h!("Accept-Charset"));
    }
    if resource.encodings_provided.len() > 1 {
        vary_header.push(h!("Accept-Encoding"));
    }
    if resource.produces.len() > 1 {
        vary_header.push(h!("Accept"));
    }

    if vary_header.len() > 1 {
        context
            .response
            .add_header("Vary", vary_header.iter().cloned().unique().collect());
    }

    if context.request.is_get_or_head() {
        {
            let callback = resource.generate_etag.lock().await;
            match callback.deref()(context, resource).await {
                Some(etag) => context
                    .response
                    .add_header("ETag", vec![HeaderValue::basic(&etag).quote()]),
                None => (),
            }
        }
        {
            let callback = resource.expires.lock().await;
            match callback.deref()(context, resource).await {
                Some(datetime) => context.response.add_header(
                    "Expires",
                    vec![HeaderValue::basic(datetime.to_rfc2822()).quote()],
                ),
                None => (),
            }
        }
        {
            let callback = resource.last_modified.lock().await;
            match callback.deref()(context, resource).await {
                Some(datetime) => context.response.add_header(
                    "Last-Modified",
                    vec![HeaderValue::basic(datetime.to_rfc2822()).quote()],
                ),
                None => (),
            }
        }
    }

    if context.response.body.is_none() && context.response.status == 200 && context.request.is_get()
    {
        let callback = resource.render_response.lock().await;
        match callback.deref()(context, resource).await {
            Some(body) => context.response.body = Some(body.into_bytes()),
            None => (),
        }
    }

    match &resource.finalise_response {
        Some(callback) => {
            let callback = callback.lock().await;
            callback.deref()(context, resource);
        }
        None => (),
    }

    debug!("Final response: {:?}", context.response);
}

#[cfg(test)]
mod tests;
