use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_postgres::{error::SqlState, Row};

use crate::utils::error::CodeHarmonyResponseError;

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login)
        .service(logout)
        .service(register)
        .service(check_logged_in);
}

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
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
        .map_err(|err| match err.as_db_error() {
            Some(err) => match *err.code() {
                SqlState::UNIQUE_VIOLATION => {
                    CodeHarmonyResponseError::BadRequest(0, "User already exists".to_string())
                }
                _ => CodeHarmonyResponseError::DatabaseConnection,
            },
            None => CodeHarmonyResponseError::DatabaseConnection,
        })?;

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

#[cfg(test)]
mod tests {
    use actix_web::{http::Method, test, App};

    use super::*;

    use crate::{create_cookie_session, create_postgres_pool};

    const USERNAME: &str = "username_test";
    const PASSWORD: &str = "password_test";

    #[actix_web::test]
    async fn test_register() {
        let app = test::init_service(
            App::new()
                .app_data(create_cookie_session())
                .app_data(web::Data::new(create_postgres_pool()))
                .service(register),
        )
        .await;

        let login_details: RegisterData = RegisterData {
            username: USERNAME.to_owned(),
            password: PASSWORD.to_owned(),
            email: "testy@testmail.com".to_owned(),
        };

        // Create request with jsonified login_details
        let req = test::TestRequest::with_uri("/account/register")
            .method(Method::POST)
            .set_json(login_details)
            .to_request();

        // Call the endpoint
        let resp = test::call_service(&app, req).await;

        // Get status and body
        let status = resp.status();
        let body = format!("{:?}", resp.into_body());

        println!("{}", body);

        // If the request was a success or if the user already exists it's good!
        assert!(status.is_success() || body.contains("User already exists"));
    }

    #[actix_web::test]
    async fn test_login_logout() {
        let app = test::init_service(
            App::new()
                .app_data(create_cookie_session())
                .app_data(web::Data::new(create_postgres_pool()))
                .service(login)
                .service(logout),
        )
        .await;

        let login_details: LoginData = LoginData {
            username: USERNAME.to_owned(),
            password: PASSWORD.to_owned(),
        };

        // Create request with jsonified login_details
        let req = test::TestRequest::with_uri("/account/login")
            .method(Method::POST)
            .set_json(login_details)
            .to_request();

        // Call the login endpoint
        let login_resp = test::call_service(&app, req).await;
        let login_status = login_resp.status();
        println!("{:?}", login_resp.into_body());

        // Create logout request
        let req = test::TestRequest::with_uri("/account/logout")
            .method(Method::POST)
            .to_request();

        // Call the logout endpoint
        let logout_resp = test::call_service(&app, req).await;
        let logout_status = logout_resp.status();
        println!("{:?}", logout_resp.into_body());

        // Check both requests went well!
        assert!(login_status.is_success() && logout_status.is_success());
    }
}
