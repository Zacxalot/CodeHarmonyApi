use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use serde::Deserialize;
use serde_json::json;
use tokio_postgres::Row;

use crate::error::CodeHarmonyResponseError;

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login)
        .service(logout)
        .service(register)
        .service(check_logged_in);
}

#[derive(Deserialize)]
struct LoginData {
    username: String,
    password: String,
}

#[derive(pg_mapper::TryFromRow)]
struct LoginDBData {
    username: String,
    hash: String,
}

// Login
#[post("/account/login")]
async fn login(
    payload: web::Json<LoginData>,
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the username and hash
    const STATEMENT: &str = "SELECT username, hash FROM codeharmony.users WHERE username=$1";
    let rows: Vec<Row> = client
        .query(STATEMENT, &[&payload.username])
        .await
        .map_err(|_| CodeHarmonyResponseError::BadRequest(0, "User not found".to_owned()))?;

    // If the user exists and could be deserialized
    // Verify the hash and login if it's correct
    if let Some(Ok(user_data)) = rows.into_iter().map(LoginDBData::try_from).next() {
        let parsed_hash = PasswordHash::new(&user_data.hash).map_err(|_| {
            CodeHarmonyResponseError::InternalError(1, "Couldn't decode password hash".to_owned())
        })?;

        Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .map_err(|_| {
                CodeHarmonyResponseError::BadRequest(3, "Incorrect password".to_owned())
            })?;

        session
            .insert("username", &user_data.username)
            .map_err(|_| {
                CodeHarmonyResponseError::InternalError(5, "Couldn't save session".to_owned())
            })?;

        Ok(HttpResponse::Ok())
    } else {
        Err(CodeHarmonyResponseError::BadRequest(
            0,
            "User not found".to_owned(),
        ))
    }
}

// Completes logout
#[post("/account/logout")]
async fn logout(session: Session) -> Result<impl Responder, CodeHarmonyResponseError> {
    session.remove("username");
    Ok(HttpResponse::Ok())
}

#[derive(Deserialize)]
struct RegisterData {
    username: String,
    password: String,
    email: String,
}

// Register
#[post("/account/register")]
async fn register(
    payload: web::Json<RegisterData>,
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Generate hash for password
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| {
            CodeHarmonyResponseError::InternalError(0, "Couldn't hash password".to_owned())
        })?
        .to_string();

    // Insert user into database
    const STATEMENT: &str = "INSERT INTO codeharmony.users(username,hash,email) VALUES($1,$2,$3)";
    client
        .query(
            STATEMENT,
            &[&payload.username, &password_hash, &payload.email],
        )
        .await
        .map_err(|_| CodeHarmonyResponseError::InternalError(1, "Couldn't register".to_owned()))?;

    // Log the user in
    session.insert("username", &payload.username).map_err(|_| {
        CodeHarmonyResponseError::InternalError(
            2,
            "Couldn't save session, account still created".to_owned(),
        )
    })?;

    Ok(HttpResponse::Ok())
}

// Check login
#[get("/account/check")]
async fn check_logged_in(session: Session) -> Result<impl Responder, CodeHarmonyResponseError> {
    // If the user exists, echo
    if let Ok(Some(username)) = session.get::<String>("username") {
        Ok(HttpResponse::Ok().json(json!({ "username": username })))
    } else {
        Err(CodeHarmonyResponseError::InternalError(
            0,
            "Not logged in".to_owned(),
        ))
    }
}
