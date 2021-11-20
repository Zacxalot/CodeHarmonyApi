use std::{convert::{TryFrom, TryInto}, error::Error, fmt::Display};

use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder, Responder, error, get, http::{StatusCode, header}, post, web};
use deadpool_postgres::{Pool};
use derive_more::{Display};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use serde_json::{Value};
use mime;

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
    elements:Vec<CodingLessonElement>
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



#[derive(Deserialize, Serialize,Debug)]
struct CodingLessonElement{
    el_type:ElementType,
    props:Vec<Value>,
    children:Vec<CodingLessonElement>
}

#[derive(Serialize, Deserialize,Debug)]
enum ElementType{
    div,
    h1
}

impl ElementType{
    fn from_string(string: &str) -> Result<ElementType,&str>{
        match string {
            "div" => return Ok(ElementType::div),
            "h1" => return Ok(ElementType::h1),
            _ => Err("Invalid element type")
        }
    }
}





// Request Payloads
#[derive(Deserialize)]
struct NewPlanRequest {
    plan_name:String
}

#[derive(Deserialize)]
struct PlanInfoRequest {
    plan_name:String
}

// Request a new plan
#[post("/plan/new")]
async fn create_lesson_plan(payload: web::Json<NewPlanRequest>, db_pool: web::Data<Pool>) -> impl Responder {

    // Get db client
    let mut client = match db_pool.get().await{
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-1".to_string(),msg:"Couldn't connect to server".to_string()})
    };

    // Start transaction
    let transaction = match client.transaction().await{
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-2".to_string(),msg:"Couldn't create transaction".to_string()})
    };

    // Try to insert the new plan into to the db
    match transaction.query("INSERT INTO codeharmony.lesson_plan(username,plan_name) VALUES ('user1',$1)",&[&payload.plan_name]).await{
        Ok(_) => (),
        Err(err) => {
            match err.as_db_error(){
                Some(err) => {
                    if err.message().starts_with("duplicate key"){
                        return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-3".to_string(),msg:"Plan name already in use".to_string()})
                    }
                    else if err.message().starts_with("value too long") {
                        return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-4".to_string(),msg:"Plan name too long".to_string()})
                    }
                    else{
                        return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-5".to_string(),msg:"Error creating new lesson plan".to_string()})
                    }
                },
                None => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-5".to_string(),msg:"Error creating new lesson plan".to_string()})
            };
        }
    };

    // Get the resulting name of the plan
    let inserted_plan_nanme:String = match transaction.query("SELECT plan_name FROM codeharmony.lesson_plan WHERE plan_name=$1",&[&payload.plan_name]).await{
        Ok(rows) => rows[0].get(0),
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-5".to_string(),msg:"Error creating new lesson plan".to_string()})
    };

    // Commit transaction, everything went well
    match transaction.commit().await{
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-5".to_string(),msg:"Error creating new lesson plan".to_string()})
    };

    // Return ok with the new plan name
    HttpResponse::Ok().json(NewPlanResponse {
        plan_name:inserted_plan_nanme,
        msg:String::new()
    })
}

// Get list of plans for dashboard
#[get("/plan/list")]
async fn get_plan_list(db_pool: web::Data<Pool>) -> impl Responder {
    // Get db client
    let client = match db_pool.get().await{
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-1".to_string(),msg:"Couldn't connect to server".to_string()})
    };

    // Get the list of plans from db
    let plan_list:Vec<Row> = match client.query("SELECT plan_name FROM codeharmony.lesson_plan WHERE username='user1'",&[]).await{
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-5".to_string(),msg:"Error creating new lesson plan".to_string()})
    };

    

    // Return list of plans
    HttpResponse::Ok().json(PlanListResponse {
        plans:plan_list.iter().map(|x| PlanInfoListItem{plan_name:x.get(0)}).collect()
    })
}

#[derive(Error,Debug)]
enum CodeHarmonyResponseError {
    #[error("Gen")]
    GenericInternalError,

    #[error("notgood")]
    InternalError(String)
}

impl error::ResponseError for CodeHarmonyResponseError{
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            CodeHarmonyResponseError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CodeHarmonyResponseError::GenericInternalError => StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .insert_header(header::ContentType(mime::TEXT_HTML_UTF_8))
            .body(self.to_string())
    }
}


// Get all associated information about a plan
#[get("/plan/info/{plan_name}")]
async fn get_plan_info(db_pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder,CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => return Err(CodeHarmonyResponseError::InternalError("bad".to_string()))
    };

    // Get db client
    let client = db_pool.get().await.map_err(|_| CodeHarmonyResponseError::InternalError("Test".to_string()))?;

    // Get the list of plans from db
    let section_list:Vec<Row> = client.query("SELECT section_name, section_type, section_elements, coding_data FROM codeharmony.lesson_plan_section WHERE plan_name=$1 and username='user1'",&[&plan_name]).await
                                      .map_err(|_| CodeHarmonyResponseError::InternalError("Test".to_string()))?;
    
    // Return list of plans
    Ok(HttpResponse::Ok().json(
        section_list.iter().map(|x| PlanSection::try_from(x)).collect::<Result<Vec<_>,_>>().map_err(|_| CodeHarmonyResponseError::InternalError("Test".to_string()))?
    ))
}