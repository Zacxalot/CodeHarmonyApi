use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder};

use crate::error::CodeHarmonyResponseError;

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_teachers);
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
