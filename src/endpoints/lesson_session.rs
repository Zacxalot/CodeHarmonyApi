use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::NaiveDateTime;
use deadpool_postgres::{Object, Pool};
use deadpool_redis::redis::cmd;
use futures::join;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_postgres::error::SqlState;

use crate::{lesson_plan::get_plan_info_query, utils::error::CodeHarmonyResponseError};

#[derive(Serialize)]
struct SessionInfo {
    date: i64,
}

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(create_session)
        .service(get_session_info)
        .service(start_session)
        .service(save_code)
        .service(get_active_sessions_for_user);
}

// Request a new session
#[post("/session/new/{plan_name}/{session_name}")]
async fn create_session(
    db_pool: web::Data<Pool>,
    path: web::Path<(String, String)>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get vars from path
    let (plan_name, session_name) = path.into_inner();

    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let mut client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Try to insert the new plan into to the db
        client
    .query(
        "INSERT INTO codeharmony.lesson_session(plan_name,session_name,username) VALUES ($1,$2,$3)",
        &[&plan_name, &session_name,&username],
    )
    .await
    .map_err(|err| match err.as_db_error() {
        Some(err) => match *err.code() {
            SqlState::UNIQUE_VIOLATION => CodeHarmonyResponseError::BadRequest(
                0,
                "Session already exists under this name".to_string(),
            ),
            _ => CodeHarmonyResponseError::DatabaseConnection,
        },
        None => CodeHarmonyResponseError::DatabaseConnection,
    })?;

        return Ok(
            HttpResponse::Ok().json(json!({"plan_name":plan_name,"session_name":session_name}))
        );
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Get info about a specific session
#[get("session/info/{plan_name}/{session_name}/{teacher_name}")]
async fn get_session_info(
    db_pool: web::Data<Pool>,
    path: web::Path<(String, String, String)>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get vars from path
    let (plan_name, session_name, teacher_name) = path.into_inner();

    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get data
    let plan_future = get_plan_info_query(&client, &plan_name, &teacher_name);
    let session_future = get_session_info_query(&client, &plan_name, &session_name, &teacher_name);

    // Wait for both futures at once
    let (plan_info_result, session_info_result) = join!(plan_future, session_future);

    // Get values from results
    let plan_info = plan_info_result?;
    let session_info = session_info_result?;

    Ok(HttpResponse::Ok().json(json!({"plan":plan_info,
                                "session":session_info})))
}

// Get session infro
async fn get_session_info_query(
    client: &Object,
    plan_name: &str,
    session_name: &str,
    username: &str,
) -> Result<SessionInfo, CodeHarmonyResponseError> {
    // Get rows from database
    const STATEMENT:&str = "SELECT session_date FROM codeharmony.lesson_session WHERE plan_name=$1 and session_name=$2 and username=$3";
    let rows = client
        .query(STATEMENT, &[&plan_name, &session_name, &username])
        .await
        .map_err(|_| {
            CodeHarmonyResponseError::InternalError(
                1,
                "Couldn't get rows from database".to_string(),
            )
        })?;

    let row = rows.first().ok_or_else(|| {
        CodeHarmonyResponseError::InternalError(2, "Could not find session".to_string())
    })?;

    let date: NaiveDateTime = row.try_get(0).map_err(|_| {
        CodeHarmonyResponseError::InternalError(3, "Couldn't parse date".to_string())
    })?;

    Ok(SessionInfo {
        date: date.timestamp_millis(),
    })
}

// Start a session
#[post("session/start/{plan_name}/{session_name}/{section_name}")]
async fn start_session(
    redis_pool: web::Data<deadpool_redis::Pool>,
    path: web::Path<(String, String, String)>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get vars from path
    let (plan_name, session_name, section_name) = path.into_inner();

    // Get the Redis client
    let mut client = redis_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Add the session to the Redis
    cmd("SET")
        .arg(&[
            "session:hosts:user1",
            "plan_name",
            &plan_name,
            "session_name",
            &session_name,
            "section_name",
            &section_name,
        ])
        .query_async(&mut client)
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return Ok!
    Ok(HttpResponse::Ok())
}

#[derive(Deserialize)]
struct SaveCode {
    text: String,
}

// Save code to redis
#[post("session/save/{host}/{plan_name}/{session_name}/{section_name}")]
async fn save_code(
    redis_pool: web::Data<deadpool_redis::Pool>,
    path: web::Path<(String, String, String, String)>,
    payload: web::Json<SaveCode>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get vars from path
    let (host, plan_name, session_name, section_name) = path.into_inner();

    // Get the Redis client
    let mut client = redis_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Save code to hashset
    cmd("HSET")
        .arg(&[
            &format!(
                "session:sessions:{}:{}:{}:{}:student1",
                host, plan_name, session_name, section_name
            ),
            "solution",
            &payload.text,
        ])
        .query_async(&mut client)
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return Ok!
    Ok(HttpResponse::Ok())
}

#[derive(pg_mapper::TryFromRow, Serialize)]
struct ActiveSession {
    session_name: String,
    plan_name: String,
    username: String,
    session_date: NaiveDateTime,
}

// Get active sessions for student
#[get("session/active")]
async fn get_active_sessions_for_user(
    db_pool: web::Data<Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT:&str = "SELECT session_name,plan_name,username,session_date FROM codeharmony.lesson_session
LEFT JOIN codeharmony.student_teacher ON codeharmony.lesson_session.username = codeharmony.student_teacher.teacher_un 
WHERE codeharmony.student_teacher.student_un = $1 OR codeharmony.lesson_session.username = $1";

        // Get rows from database
        let rows = client.query(STATEMENT, &[&username]).await.map_err(|e| {
            println!("{:?}", e);
            CodeHarmonyResponseError::InternalError(
                1,
                "Couldn't get rows from database".to_string(),
            )
        })?;

        let sessions = rows
            .into_iter()
            .map(ActiveSession::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(0, "Invalid rows".to_string())
            })?;

        // Return Ok!
        return Ok(HttpResponse::Ok().json(sessions));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}
