use crate::utils::error::CodeHarmonyResponseError;
use actix_web::{http::header, post, web, HttpResponse, Responder};
use awc::Client;
use serde::{Deserialize, Serialize};
use std::env;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(execute_code);
}

#[derive(Deserialize, Serialize)]
struct CodeFile {
    name: String,
    content: String,
}

#[derive(Deserialize, Serialize)]
struct PistonRequest {
    language: String,
    version: String,
    files: Vec<CodeFile>,
}

#[derive(Deserialize, Serialize, Debug)]
struct RunData {
    stdout: String,
    stderr: String,
    code: usize,
    output: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct PistonResponse {
    run: RunData,
    language: String,
    version: String,
}

// Request a new plan
#[post("/run")]
async fn execute_code(
    payload: web::Json<PistonRequest>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    let piston_host = env::var("PISTON_HOST").unwrap_or_else(|_| "http://127.0.0.1:2000".into());

    let client = Client::default();

    let request_data = serde_json::to_string(&payload).map_err(|_| {
        CodeHarmonyResponseError::BadRequest(0, "Couldn't parse request data".to_owned())
    })?;

    println!("{}", request_data);

    let mut response = client
        .post(format!("{}/api/v2/execute", piston_host))
        .append_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
        .send_body(request_data)
        .await
        .map_err(|e| {
            println!("{:?}", e);
            CodeHarmonyResponseError::InternalError(1, "Couldn't execute code".to_owned())
        })?;

    let body = response.json::<PistonResponse>().await.map_err(|e| {
        println!("{:?}", e);
        CodeHarmonyResponseError::InternalError(1, "Couldn't decode body".to_owned())
    })?;

    println!("{:?}", body);

    Ok(HttpResponse::Ok().json(body))
}
