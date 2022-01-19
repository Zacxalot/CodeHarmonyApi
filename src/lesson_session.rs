use actix_web::{Responder, web, HttpResponse, HttpRequest, post, get};
use chrono::NaiveDateTime;
use deadpool_postgres::{Pool, Object};
use pct_str::PctStr;
use serde::Serialize;
use serde_json::json;
use futures::join;

use crate::{error::CodeHarmonyResponseError, lesson_plan};

#[derive(Serialize)]
struct SessionInfo{
    date:i64
}

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig){
    cfg.service(create_session)
    .service(get_session_info);
}


// Request a new session
#[post("/session/new/{plan_name}/{session_name}")]
async fn create_session(db_pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder,CodeHarmonyResponseError> {
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


// Get info about a specific session
#[get("session/info/{plan_name}/{session_name}")]
async fn get_session_info(db_pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder,CodeHarmonyResponseError> {
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

    // Get data
    let plan_future = lesson_plan::get_plan_info_query(&client,&plan_name);
    let session_future = get_plan_info_query(&client,&plan_name,&session_name);

    // Wait for both futures at once
    let (plan_info_result,session_info_result) = join!(plan_future,session_future);

    // Get values from results
    let plan_info = plan_info_result?;
    let session_info = session_info_result?;

    Ok(HttpResponse::Ok().json(json!({"plan":plan_info,
                                "session":session_info})))
}

async fn get_plan_info_query(client:  &Object,plan_name:&str,session_name:&str) -> Result<SessionInfo,CodeHarmonyResponseError>{
    // Get rows from database
    let rows = client.query("SELECT session_date FROM codeharmony.lesson_session WHERE plan_name=$1 and session_name=$2 and username=$3",&[&plan_name,&session_name,&"user1"]).await
                              .map_err(|_| CodeHarmonyResponseError::InternalError(1,"Couldn't get rows from database".to_string()))?;

    let row = rows.first().ok_or(CodeHarmonyResponseError::InternalError(2,"Could not find session".to_string()))?;

    let date:NaiveDateTime = row.try_get(0)
                      .map_err(|_| CodeHarmonyResponseError::InternalError(3,"Couldn't parse date".to_string()))?;

    Ok(SessionInfo{date:date.timestamp_millis()})
}