use crate::{endpoints::lesson_plan::CodingData, utils::error::CodeHarmonyResponseError};
use actix_session::Session;
use actix_web::{http::header, post, web, HttpResponse, Responder};
use awc::Client;
use serde::{Deserialize, Serialize};
use std::{env, fmt::Display};
use tokio_postgres::types::Json;

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
pub struct PistonResponse {
    run: RunData,
    language: String,
    version: String,
}

// Display for piston response it to turn it into json
// Makes returning the error easier
impl Display for PistonResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(json) = serde_json::to_string(self) {
            return write!(f, "{}", json);
        }
        Err(std::fmt::Error)
    }
}

#[derive(Deserialize)]
struct RunRequest {
    piston: PistonRequest,
    identifier: SectionIdentifier,
}

#[derive(Deserialize)]
pub struct SectionIdentifier {
    pub plan_name: String,
    pub section_name: String,
    pub host: String,
}

// Request a new plan
#[post("/run")]
async fn execute_code(
    payload: web::Json<RunRequest>,
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Setup awc client
        let piston_host =
            env::var("PISTON_HOST").unwrap_or_else(|_| "http://127.0.0.1:2000".into());

        let client = Client::default();

        // Stringify json data
        let request_data = serde_json::to_string(&payload.piston).map_err(|_| {
            CodeHarmonyResponseError::BadRequest(0, "Couldn't parse request data".to_owned())
        })?;

        //  Make request to piston API
        let mut response = client
            .post(format!("{}/api/v2/execute", piston_host))
            .append_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .send_body(request_data)
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(1, "Couldn't execute code".to_owned())
            })?;

        // Parse json body into struct
        let body = response.json::<PistonResponse>().await.map_err(|e| {
            println!("{:?}", e);
            CodeHarmonyResponseError::InternalError(1, "Couldn't decode body".to_owned())
        })?;

        // Get database client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Query db for requried coding data
        const STATEMENT:&str =
            "SELECT coding_data FROM codeharmony.lesson_plan_section lps 
            JOIN codeharmony.student_teacher st ON lps.username=st.teacher_un
            WHERE (student_un = $1 OR teacher_un = $1) AND plan_name = $2 AND section_name = $3 AND teacher_un = $4";
        let rows = client
            .query(
                STATEMENT,
                &[
                    &username,
                    &payload.identifier.plan_name,
                    &payload.identifier.section_name,
                    &payload.identifier.host,
                ],
            )
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseQueryFailed)?;

        // Get the coding_data from the row
        if let Some(row) = rows.into_iter().next() {
            if let Ok(json) = row.try_get::<usize, Json<CodingData>>(0) {
                // If we get the expected output, return ok with the body
                if body.run.stdout.trim() == json.0.expectedOutput {
                    return Ok(HttpResponse::Accepted().json(body));
                } else {
                    // Return error with body if it's wrong
                    return Err(CodeHarmonyResponseError::IncorrectAnswer(body));
                }
            }
        }

        // Couldn't find coding data
        return Err(CodeHarmonyResponseError::NotFound);
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}
