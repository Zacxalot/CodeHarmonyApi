use actix_web::{
    error,
    http::{header, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum CodeHarmonyResponseError {
    #[error("{{\"errcode\": {0}, \"msg\": \"{1}\"}}")]
    InternalError(i32, String),
    #[error("{{\"errcode\": {0}, \"msg\": \"{1}\"}}")]
    BadRequest(i32, String),
    #[error("{{\"errcode\": 0, \"msg\": \"Couldn't connect to database\"}}")]
    DatabaseConnection,
    #[error("{{\"errcode\": 0, \"msg\": \"Couldn't connect to Redis\"}}")]
    RedisConnection,
    #[error("{{\"errcode\":401, \"msg\": \"Not logged in \"}}")]
    NotLoggedIn,
    #[error("{{\"errcode\":900, \"msg\": \"Database query failed \"}}")]
    DatabaseQueryFailed,
    #[error("{{\"errcode\":901, \"msg\": \"Couldn't parse rows\"}}")]
    CouldntParseRows,
}

impl error::ResponseError for CodeHarmonyResponseError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            CodeHarmonyResponseError::InternalError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::BadRequest(_, _) => StatusCode::BAD_REQUEST,
            CodeHarmonyResponseError::DatabaseConnection => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::RedisConnection => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::NotLoggedIn => StatusCode::UNAUTHORIZED,
            CodeHarmonyResponseError::DatabaseQueryFailed => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::CouldntParseRows => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
