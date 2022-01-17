use std::{convert::TryFrom};
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web, put};
use deadpool_postgres::{Pool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_postgres::Row;
use tokio_postgres::error::SqlState;
use crate::error::CodeHarmonyResponseError;
use crate::jsx_element::{JSXElement};


//Responses
#[derive(Serialize)]
struct NewPlanResponse {
    plan_name:String,
    msg:String
}

#[derive(Serialize)]
struct PlanInfoListItem{
    plan_name:String
}

#[derive(Serialize)]
struct PlanListResponse{
    plans:Vec<PlanInfoListItem>
}

#[derive(Serialize)]
struct PlanSectionListResponse{
    sections:Vec<PlanSection>
}


#[derive(Serialize,Deserialize,Debug)]
struct PlanSection{
    name:String,
    section_type:String,
    elements:Vec<JSXElement>,
    order_pos:i16
}

#[derive(Serialize,Deserialize,Debug)]
struct PlanOperation{
    request:String,
    data:Value
}

#[derive(Deserialize,Debug)]
struct NewSectionData{
    section_name:String,
    order_pos:i16
}

#[derive(Serialize)]
struct SessionListItem{
    plan_name:String,
    session_name:String
}

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig){
    cfg.service(create_lesson_plan)
    .service(get_plan_list)
    .service(get_plan_info)
    .service(set_plan_section)
    .service(perform_plan_operation);
}


impl TryFrom<&tokio_postgres::Row> for PlanSection{
    type Error = Box<dyn std::error::Error>;
    fn try_from(row: &tokio_postgres::Row) -> Result<Self, Self::Error> {
        let cols = row.columns();

        println!("{:?}",cols);

        if cols.len() >= 5 && cols.get(0).unwrap().name() == "section_name" && cols.get(1).unwrap().name() == "section_type" && cols.get(2).unwrap().name() == "section_elements" && cols.get(4).unwrap().name() == "order_pos"{
            return Ok(
                PlanSection{
                    name:row.try_get::<usize,String>(0)?,
                    section_type:row.try_get::<usize,String>(1)?,
                    elements:serde_json::from_value(row.try_get::<usize,serde_json::Value>(2)?)?,
                    order_pos:row.try_get::<usize,i16>(4)?
                }
            );
        }
        Err(Box::from("Invalid Rows"))
    }
}

// Request Payloads
#[derive(Deserialize)]
struct NewPlanRequest {
    plan_name:String
}

// Request a new plan
#[post("/plan/new")]
async fn create_lesson_plan(payload: web::Json<NewPlanRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder,CodeHarmonyResponseError> {

    // Get db client
    let mut client = db_pool.get().await.map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Start transaction
    let transaction = client.transaction().await.map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Try to insert the new plan into to the db
    transaction.query("INSERT INTO codeharmony.lesson_plan(username,plan_name) VALUES ('user1',$1)",&[&payload.plan_name]).await
               .map_err(|err| match err.as_db_error(){
                   Some(err) => {
                       match *err.code(){
                           SqlState::UNIQUE_VIOLATION => CodeHarmonyResponseError::BadRequest(0,"Plan already exists under this name".to_string()),
                           _ => CodeHarmonyResponseError::DatabaseConnection
                       }},
                   None => CodeHarmonyResponseError::DatabaseConnection
               })?;

    // Get the resulting name of the plan
    let inserted_plan_name:String = transaction.query("SELECT plan_name FROM codeharmony.lesson_plan WHERE plan_name=$1",&[&payload.plan_name]).await
                                                .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?
                                                [0].get::<usize,String>(0);

    // Commit transaction, everything went well
    transaction.commit().await.map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return ok with the new plan name
    Ok(HttpResponse::Ok().json(NewPlanResponse {
        plan_name:inserted_plan_name,
        msg:String::new()
    }))
}

// Get list of plans for dashboard
#[get("/plan/list")]
async fn get_plan_list(db_pool: web::Data<Pool>) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool.get().await.map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of plans from db
    let plan_list:Vec<Row> = client.query("SELECT plan_name FROM codeharmony.lesson_plan WHERE username='user1'",&[]).await
                                   .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of sessions from db
    let session_list:Vec<Row> = client.query("SELECT plan_name,session_name FROM codeharmony.lesson_session WHERE username='user1'",&[]).await
                                   .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return list of plans
    Ok(HttpResponse::Ok().json(json!({
        "plans":plan_list.iter().map(|x| PlanInfoListItem{plan_name:x.get(0)}).collect::<Vec<PlanInfoListItem>>(),
        "sessions":session_list.iter().map(|row| SessionListItem{plan_name:row.get(0),session_name:row.get(1)}).collect::<Vec<SessionListItem>>()
    })))
}


// Get all associated information about a plan
#[get("/plan/info/{plan_name}")]
async fn get_plan_info(db_pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => return Err(CodeHarmonyResponseError::BadRequest(0,"Expected plan name in uri".to_string()))
    };

    // Get db client
    let client = db_pool.get().await
                                      .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of plans from db
    let section_list:Vec<Row> = client.query("SELECT section_name, section_type, section_elements, coding_data, order_pos FROM codeharmony.lesson_plan_section WHERE plan_name=$1 and username='user1' ORDER BY order_pos ASC",&[&plan_name]).await
                                      .map_err(|_| CodeHarmonyResponseError::InternalError(1,"Couldn't get rows from database".to_string()))?;
    
    // Return list of plans
    Ok(HttpResponse::Ok().json(
        section_list.iter()
                    .map(|x| PlanSection::try_from(x))
                    .collect::<Result<Vec<_>,_>>()
                    .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(2,"Invalid row format!".to_string())})?
    ))
}

// Update the json data for a plan section
#[put("/plan/info/{plan_name}")]
async fn set_plan_section(db_pool: web::Data<Pool>, req: HttpRequest, section: web::Json<PlanSection>) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => return Err(CodeHarmonyResponseError::BadRequest(0,"Expected plan name in uri".to_string()))
    };

    // Get db client
    let client = db_pool.get().await
                                      .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the body as a json string
    let section_json = serde_json::to_value(&section.elements)
                                         .map_err(|_| CodeHarmonyResponseError::BadRequest(1,"Couldn't serialise json".to_string()))?;

    // Send the update query with the new section json data
    client.query("UPDATE codeharmony.lesson_plan_section SET section_elements = $1, order_pos = $2 WHERE plan_name = $3 and section_name=$4 and username='user1'",&[&section_json,&section.order_pos,&plan_name,&section.name]).await
          .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(1,"Couldn't update database".to_string())})?;
    
    // Return Ok!
    Ok(HttpResponse::Ok())
}


// Add new section to plan
#[post("/plan/info/{plan_name}")]
async fn perform_plan_operation(db_pool: web::Data<Pool>, req: HttpRequest, operation: web::Json<PlanOperation>) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => return Err(CodeHarmonyResponseError::BadRequest(0,"Expected plan name in uri".to_string()))
    };


    // Get db client
    let client = db_pool.get().await
                                      .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;


    if &operation.request == "new-section"{
        let data:NewSectionData = serde_json::from_value(operation.data.to_owned()).map_err(|_| CodeHarmonyResponseError::BadRequest(1,"Invalid operation data".to_string()))?;

        client.query("INSERT INTO codeharmony.lesson_plan_section(plan_name,username,order_pos,section_name,section_type) VALUES($1,$2,$3,$4,$5)",&[&plan_name,&"user1",&data.order_pos,&data.section_name,&"LECTURE"]).await
              .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(2,"Couldn't add section".to_string())})?;
    }

    Ok(HttpResponse::Ok())
}