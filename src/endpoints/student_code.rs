use std::convert::TryFrom;

use crate::utils::error::CodeHarmonyResponseError;
use actix_session::Session;
use actix_web::{get, http::header::ContentType, post, web, HttpResponse, Responder};
use deadpool_postgres::Pool;
use pg_mapper::TryFromRow;
use serde::Serialize;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(save_code)
        .service(get_code)
        .service(get_submitted_list)
        .service(get_student_code);
}

// Save code to db
#[post("session/save/{plan_name}/{session_name}/{host}/{section_name}")]
async fn save_code(
    path: web::Path<(String, String, String, String)>,
    code: String,
    session: Session,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Check code length
        if code.len() > 10000 {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Code too long!".to_owned(),
            ));
        }

        // Get vars from path
        let (plan_name, session_name, host, section_name) = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT: &str = "
            INSERT INTO codeharmony.code_submission(teacher_un, plan_name, section_name, session_name, student_un, code)
            VALUES($1,$2,$3,$4,$5,$6)
            ON CONFLICT ON CONSTRAINT code_submission_pk
            DO UPDATE SET code=$6
        ";

        // Do insert query
        client
            .query(
                STATEMENT,
                &[
                    &host,
                    &plan_name,
                    &section_name,
                    &session_name,
                    &username,
                    &code,
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseConnection
            })?;

        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Get code from db
#[get("session/save/{plan_name}/{session_name}/{host}/{section_name}")]
async fn get_code(
    db_pool: web::Data<Pool>,
    path: web::Path<(String, String, String, String)>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get vars from path
        let (plan_name, session_name, host, section_name) = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT: &str = "SELECT code FROM codeharmony.code_submission WHERE teacher_un = $1 AND plan_name = $2 AND section_name = $3 AND session_name = $4 AND student_un = $5";

        let rows = client
            .query(
                STATEMENT,
                &[&host, &plan_name, &section_name, &session_name, &username],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::InternalError(
                    1,
                    "Couldn't get rows from database".to_string(),
                )
            })?;

        if let Some(row) = rows.first() {
            if let Ok(code) = row.try_get::<usize, String>(0) {
                // Return Ok!
                return Ok(HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(code));
            }
        }

        return Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .body("[]"));
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[derive(TryFromRow, Serialize)]
struct SubmittedList {
    student_un: String,
    correct: bool,
}

#[get("session/submitted/{plan_name}/{session_name}/{section_name}")]
async fn get_submitted_list(
    db_pool: web::Data<Pool>,
    path: web::Path<(String, String, String)>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        let (plan_name, session_name, section_name) = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT: &str = "SELECT student_un, correct FROM codeharmony.code_submission WHERE teacher_un = $1 AND plan_name = $2 AND section_name = $3 AND session_name = $4";

        let rows = client
            .query(
                STATEMENT,
                &[&username, &plan_name, &section_name, &session_name],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::InternalError(
                    1,
                    "Couldn't get rows from database".to_string(),
                )
            })?;

        let submitted = rows
            .into_iter()
            .map(SubmittedList::try_from)
            .collect::<Result<Vec<SubmittedList>, _>>()
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::CouldntParseRows
            })?;

        return Ok(HttpResponse::Ok().json(submitted));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[get("session/submitted/{plan_name}/{session_name}/{section_name}/{student_name}")]
async fn get_student_code(
    db_pool: web::Data<Pool>,
    path: web::Path<(String, String, String, String)>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        let (plan_name, session_name, section_name, student_un) = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT: &str = "SELECT code FROM codeharmony.code_submission WHERE teacher_un = $1 AND plan_name = $2 AND section_name = $3 AND session_name = $4 AND student_un = $5";

        let rows = client
            .query(
                STATEMENT,
                &[
                    &username,
                    &plan_name,
                    &section_name,
                    &session_name,
                    &student_un,
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::InternalError(
                    1,
                    "Couldn't get rows from database".to_string(),
                )
            })?;

        if let Some(row) = rows.get(0) {
            if let Ok(code) = row.try_get::<usize, String>(0) {
                return Ok(HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(code));
            }
        }

        return Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .body("[]"));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}
