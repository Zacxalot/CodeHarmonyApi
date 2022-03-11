use actix::Addr;
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use serde_json::json;
use tokio_postgres::error::SqlState;

use crate::{
    actors::teacher_code_manager::{GetCode, GetTeacher, TeacherCodeManager},
    error::CodeHarmonyResponseError,
};

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_teachers)
        .service(create_teacher_code)
        .service(add_teacher);
}

#[get("account/teachers")]
async fn get_teachers(
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // If the user is logged in and their username is availabe
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get rows
        const STATEMENT: &str =
            "SELECT teacher_un from codeharmony.student_teacher WHERE student_un = $1";

        let rows = client
            .query(STATEMENT, &[&username])
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseQueryFailed)?;

        // For each row, try and get teacherUN, collect into option, return usernames if Some
        if let Some(parsed) = rows
            .into_iter()
            .map(|row| row.get("teacher_un"))
            .collect::<Option<Vec<String>>>()
        {
            return Ok(HttpResponse::Ok().json(parsed));
        }
        // If None, return CouldntParseRowsError
        return Err(CodeHarmonyResponseError::CouldntParseRows);
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[get("account/my-code")]
async fn create_teacher_code(
    session: Session,
    code_manager: web::Data<Addr<TeacherCodeManager>>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get the code map
        let code = code_manager.send(GetCode { username }).await.map_err(|_| {
            CodeHarmonyResponseError::InternalError(0, "Couldn't generate code".into())
        })?;
        return Ok(HttpResponse::Ok().json(json!({ "code": code })));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[post("account/add-teacher/{code}")]
async fn add_teacher(
    path: web::Path<String>,
    session: Session,
    code_manager: web::Data<Addr<TeacherCodeManager>>,
    db_pool: web::Data<deadpool_postgres::Pool>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    let code = path.into_inner();
    if let Ok(Some(username)) = session.get::<String>("username") {
        if let Ok(Some(teacher_un)) = code_manager.send(GetTeacher { code }).await {
            // If the code is valid, add the record to the DB

            // Get db client
            let client = db_pool
                .get()
                .await
                .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

            const STATEMENT: &str =
                "INSERT INTO codeharmony.student_teacher (teacher_un, student_un) VALUES ($1, $2)";

            // Insert the record,
            // Return teacher aleardy added if not unique
            client
                .query(STATEMENT, &[&teacher_un, &username])
                .await
                .map_err(|err| -> CodeHarmonyResponseError {
                    match err.as_db_error() {
                        Some(err) => match *err.code() {
                            SqlState::UNIQUE_VIOLATION => CodeHarmonyResponseError::BadRequest(
                                0,
                                "Teacher already added".into(),
                            ),
                            _ => CodeHarmonyResponseError::DatabaseQueryFailed,
                        },
                        None => CodeHarmonyResponseError::DatabaseQueryFailed,
                    }
                })?;

            return Ok(HttpResponse::Ok().json(json!({ "teacher_un": teacher_un })));
        } else {
            return Err(CodeHarmonyResponseError::InternalError(
                0,
                "Invalid code".into(),
            ));
        }
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}
