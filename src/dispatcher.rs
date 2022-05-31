use std::task;

use hyper::Body;

use super::*;

/// The main hyper dispatcher
#[derive(Clone)]
pub struct Dispatcher<'a> {
    /// Map of routes to webmachine resources
    pub routes: BTreeMap<&'a str, Resource<'a>>,
}

impl<'a> Dispatcher<'a> {
    /// Main dispatch function for the Webmachine. This will look for a matching resource
    /// based on the request path. If one is not found, a 404 Not Found response is returned
    pub async fn dispatch(self, req: http::Request<Body>) -> http::Result<http::Response<Body>> {
        let mut context = self.context_from_http_request(req).await;
        self.dispatch_to_resource(&mut context).await;
        self.generate_http_response(&context)        
    }

    async fn context_from_http_request(&self, req: http::Request<Body>) -> Context {
        let request = self.request_from_http_request(req).await;
        Context {
            request,
            response: Response::default(),
            ..Context::default()
        }
    }

    pub(crate) fn match_paths(&self, request: &Request) -> Vec<String> {
        let request_path = sanitise_path(&request.request_path);
        self.routes
            .keys()
            .filter(|k| request_path.starts_with(&sanitise_path(k)))
            .map(|k| k.to_string())
            .collect()
    }

    pub(crate) fn lookup_resource(&self, path: &str) -> Option<&Resource<'a>> {
        self.routes.get(path)
    }

    /// Dispatches to the matching webmachine resource. If there is no matching resource, returns
    /// 404 Not Found response
    pub async fn dispatch_to_resource(&self, context: &mut Context) {
        let matching_paths = self.match_paths(&context.request);
        let ordered_by_length: Vec<String> = matching_paths
            .iter()
            .cloned()
            .sorted_by(|a, b| Ord::cmp(&b.len(), &a.len()))
            .collect();
        match ordered_by_length.first() {
            Some(path) => {
                update_paths_for_resource(&mut context.request, path);
                if let Some(resource) = self.lookup_resource(path) {
                    execute_state_machine(context, &resource).await;
                    finalise_response(context, &resource).await;
                } else {
                    context.response.status = 404;
                }
            }
            None => context.response.status = 404,
        };
    }

    fn generate_http_response(&self, context: &Context) -> http::Result<http::Response<Body>> {
        let mut response = http::Response::builder().status(context.response.status);
    
        for (header, values) in context.response.headers.clone() {
            let header_values = values.iter().map(|h| h.to_string()).join(", ");
            response = response.header(&header, &header_values);
        }
    
        match context.response.body.clone() {
            Some(body) => response.body(body.into()),
            None => response.body(Body::empty()),
        }
    }

    async fn request_from_http_request(&self, req: http::Request<Body>) -> Request {
        let (parts, body) = req.into_parts();
        let request_path = parts.uri.path().to_string();
    
        let req_body = body
            .try_fold(Vec::new(), |mut data, chunk| async move {
                data.extend_from_slice(&chunk);
                Ok(data)
            })
            .await;
        let body = match req_body {
            Ok(body) => {
                if body.is_empty() {
                    None
                } else {
                    Some(body.clone())
                }
            }
            Err(err) => {
                error!("Failed to read the request body: {}", err);
                None
            }
        };
    
        let query = match parts.uri.query() {
            Some(query) => parse_query(query),
            None => HashMap::new(),
        };
        Request {
            request_path: request_path.clone(),
            base_path: "/".to_string(),
            method: parts.method.as_str().into(),
            headers: headers_from_http_request(&parts),
            body,
            query,
        }
    }
}

impl Service<http::Request<Body>> for Dispatcher<'static> {
    type Response = http::Response<Body>;
    type Error = http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        Box::pin(self.clone().dispatch(req))
    }
}
