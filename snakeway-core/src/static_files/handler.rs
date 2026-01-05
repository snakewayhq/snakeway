use http::{HeaderMap, HeaderValue, StatusCode};

use crate::route::RouteRuntime;
use crate::static_files::render::{render_directory, render_file};
use crate::static_files::resolve::{ResolveError, ResolvedStatic, resolve_static_path};
use crate::static_files::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};

pub async fn handle_static_request(
    route: &RouteRuntime,
    request_path: &str,
    conditional: &ConditionalHeaders,
) -> StaticResponse {
    let RouteRuntime::Static {
        id,
        path,
        file_dir,
        index,
        directory_listing,
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

    match resolved {
        ResolvedStatic::File(path) => render_file(path, conditional, static_config, cache_policy)
            .await
            .unwrap_or_else(|e| error_response(map_serve_error(e))),

        ResolvedStatic::Directory(dir) => {
            if !directory_listing {
                return error_response(StatusCode::FORBIDDEN);
            }

            render_directory(dir, request_path)
        }
    }
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
    let mut headers = HeaderMap::new();
    headers.insert(http::header::CONTENT_LENGTH, HeaderValue::from_static("0"));

    StaticResponse {
        status,
        headers,
        body: StaticBody::Empty,
    }
}
