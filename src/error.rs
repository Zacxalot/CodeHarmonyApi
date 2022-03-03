use actix_web::{
    error,
    http::{header, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use thiserror::Error;

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
}

impl error::ResponseError for CodeHarmonyResponseError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            CodeHarmonyResponseError::InternalError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::BadRequest(_, _) => StatusCode::BAD_REQUEST,
            CodeHarmonyResponseError::DatabaseConnection => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::RedisConnection => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
