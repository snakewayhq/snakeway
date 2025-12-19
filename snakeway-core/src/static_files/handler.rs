use http::{HeaderMap, StatusCode};

use crate::route::RouteKind;
use crate::static_files::resolve::{resolve_static_path, ResolveError};
use crate::static_files::serve::{serve_file, ConditionalHeaders, ServeError, StaticResponse};
use crate::static_files::StaticBody;

pub async fn handle_static_request(
    route: &RouteKind,
    request_path: &str,
    conditional: &ConditionalHeaders,
) -> StaticResponse {
    let RouteKind::Static {
        path,
        file_dir,
        index,
        static_config,
        cache_policy,
    } = route
    else {
        unreachable!("handle_static_request called with non-static route");
    };

    let resolved = match resolve_static_path(file_dir, path, request_path, *index) {
        Ok(p) => p,
        Err(e) => return error_response(map_resolve_error(e)),
    };

    serve_file(resolved, conditional, static_config, cache_policy)
        .await
        .unwrap_or_else(|e| error_response(map_serve_error(e)))
}

fn map_resolve_error(err: ResolveError) -> StatusCode {
    match err {
        ResolveError::NotFound => StatusCode::NOT_FOUND,
        ResolveError::Forbidden => StatusCode::FORBIDDEN,
        ResolveError::BadPath => StatusCode::BAD_REQUEST,
    }
}

fn map_serve_error(err: ServeError) -> StatusCode {
    match err {
        ServeError::NotFound => StatusCode::NOT_FOUND,
        ServeError::Forbidden => StatusCode::FORBIDDEN,
        ServeError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn error_response(status: StatusCode) -> StaticResponse {
    StaticResponse {
        status,
        headers: HeaderMap::new(),
        body: StaticBody::Empty,
    }
}
