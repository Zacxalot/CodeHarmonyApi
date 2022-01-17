use actix_web::{Responder, web, HttpResponse, HttpRequest, post, get};
use deadpool_postgres::Pool;
use pct_str::PctStr;
use serde::Serialize;
use serde_json::json;
use tokio_postgres::Row;

use crate::error::CodeHarmonyResponseError;

#[derive(Serialize)]
struct SessionListItem{
    plan_name:String,
    session_name:String
}

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig){
    cfg.service(create_session)
    .service(get_session_list);
}


// Request a new session
#[post("/session/new/{plan_name}/{session_name}")]
async fn create_session(db_pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder,CodeHarmonyResponseError> {
    println!("Creating Session");
    // Get plan name from uri decoding it as well
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => PctStr::new(plan_name).map_err(|_| CodeHarmonyResponseError::BadRequest(1,"Bad plan name".to_string()))?.decode(),
        None => return Err(CodeHarmonyResponseError::BadRequest(0,"Expected plan name in uri".to_string()))
    };

    // Get session name from uri decoding it as well
    let session_name = match req.match_info().get("session_name") {
        Some(session_name) => PctStr::new(session_name).map_err(|_| CodeHarmonyResponseError::BadRequest(1,"Bad plan name".to_string()))?.decode(),
        None => return Err(CodeHarmonyResponseError::BadRequest(0,"Expected session name in uri".to_string()))
    };

    // Get db client
    let client = db_pool.get().await
                                      .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Create session record in database
    client.query("INSERT INTO codeharmony.lesson_session(plan_name,session_name,username) VALUES ($1,$2,$3)",&[&plan_name, &session_name,&"user1"]).await
        .map_err(|e| {println!("{:?}",e); CodeHarmonyResponseError::InternalError(1,"Couldn't create lesson session".to_string())})?;


    Ok(HttpResponse::Ok().json(json!({"plan_name":plan_name,"session_name":session_name})))
}

// Get list of plans for dashboard
#[get("/session/list")]
async fn get_session_list(db_pool: web::Data<Pool>) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool.get().await.map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of plans from db
    let session_list:Vec<Row> = client.query("SELECT plan_name,session_name FROM codeharmony.lesson_session WHERE username='user1'",&[]).await
                                   .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return list of plans
    Ok(HttpResponse::Ok().json(json!({
        "sessions":session_list.iter().map(|row| SessionListItem{plan_name:row.get(0),session_name:row.get(1)}).collect::<Vec<SessionListItem>>()
    })))
}