use actix_web::{Responder, web, post, HttpResponse};
use deadpool_postgres::Pool;

use crate::error::CodeHarmonyResponseError;

// Request a new plan
#[post("/plan/new")]
async fn create_session(db_pool: web::Data<Pool>) -> Result<impl Responder,CodeHarmonyResponseError> {
    
    Ok(HttpResponse::Ok())
}