use std::{convert::TryFrom};
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use deadpool_postgres::{Pool};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use tokio_postgres::error::SqlState;
use crate::error::CodeHarmonyResponseError;
use crate::jsx_element::JSXElement;


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


#[derive(Serialize,Debug)]
struct PlanSection{
    name:String,
    section_type:String,
    elements:Vec<JSXElement>
}

impl TryFrom<&tokio_postgres::Row> for PlanSection{
    type Error = Box<dyn std::error::Error>;
    fn try_from(row: &tokio_postgres::Row) -> Result<Self, Self::Error> {
        let cols = row.columns();

        println!("{:?}",cols);

        if cols.len() >= 3 && cols.get(0).unwrap().name() == "section_name" && cols.get(1).unwrap().name() == "section_type" && cols.get(2).unwrap().name() == "section_elements"{
            return Ok(
                PlanSection{
                    name:row.try_get::<usize,String>(0)?,
                    section_type:row.try_get::<usize,String>(1)?,
                    elements:serde_json::from_value(row.try_get::<usize,serde_json::Value>(2)?)?}
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

    // Return list of plans
    Ok(HttpResponse::Ok().json(PlanListResponse {
        plans:plan_list.iter().map(|x| PlanInfoListItem{plan_name:x.get(0)}).collect()
    }))
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
    let section_list:Vec<Row> = client.query("SELECT section_name, section_type, section_elements, coding_data FROM codeharmony.lesson_plan_section WHERE plan_name=$1 and username='user1'",&[&plan_name]).await
                                      .map_err(|_| CodeHarmonyResponseError::InternalError(1,"Couldn't get rows from database".to_string()))?;
    
    // Return list of plans
    Ok(HttpResponse::Ok().json(
        section_list.iter()
                    .map(|x| PlanSection::try_from(x))
                    .collect::<Result<Vec<_>,_>>()
                    .map_err(|_| CodeHarmonyResponseError::InternalError(2,"Invalid row format!".to_string()))?
    ))
}