use crate::error::CodeHarmonyResponseError;
use crate::jsx_element::JSXElement;
use actix_web::{get, post, put, web, HttpRequest, HttpResponse, Responder};
use deadpool_postgres::{Object, Pool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::TryFrom;
use std::string;
use tokio_postgres::error::SqlState;
use tokio_postgres::Row;

//Responses
#[derive(Serialize)]
#[allow(non_snake_case)]
struct NewPlanResponse {
    planName: String,
    msg: String,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct PlanInfoListItem {
    planName: String,
}

#[derive(Serialize)]
struct PlanListResponse {
    plans: Vec<PlanInfoListItem>,
}

#[derive(Serialize)]
struct PlanSectionListResponse {
    sections: Vec<PlanSection>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct CodingData {
    language: String,
    startingCode: String,
    expectedOutput: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct PlanSection {
    name: String,
    sectionType: String,
    elements: Vec<JSXElement>,
    orderPos: i16,
    codingData: CodingData,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlanOperation {
    request: String,
    data: Value,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct NewSectionData {
    sectionName: String,
    orderPos: i16,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct SessionListItem {
    planName: String,
    sessionName: String,
}

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(create_lesson_plan)
        .service(get_plan_list)
        .service(get_plan_info)
        .service(set_plan_section)
        .service(perform_plan_operation)
        .service(update_plan_section_name)
        .service(update_plan_section_type);
}

impl TryFrom<&tokio_postgres::Row> for PlanSection {
    type Error = Box<dyn std::error::Error>;
    fn try_from(row: &tokio_postgres::Row) -> Result<Self, Self::Error> {
        let cols = row.columns();

        if cols.len() >= 5
            && cols.get(0).unwrap().name() == "section_name"
            && cols.get(1).unwrap().name() == "section_type"
            && cols.get(2).unwrap().name() == "section_elements"
            && cols.get(3).unwrap().name() == "coding_data"
            && cols.get(4).unwrap().name() == "order_pos"
        {
            return Ok(PlanSection {
                name: row.try_get::<usize, String>(0)?,
                sectionType: row.try_get::<usize, String>(1)?,
                elements: serde_json::from_value(row.try_get::<usize, serde_json::Value>(2)?)?,
                codingData: serde_json::from_value(row.try_get::<usize, serde_json::Value>(3)?)?,
                orderPos: row.try_get::<usize, i16>(4)?,
            });
        }
        Err(Box::from("Invalid Rows"))
    }
}

// Request Payloads
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct NewPlanRequest {
    planName: String,
}

// Request a new plan
#[post("/plan/new")]
async fn create_lesson_plan(
    payload: web::Json<NewPlanRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let mut client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Start transaction
    let transaction = client
        .transaction()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Try to insert the new plan into to the db
    transaction
        .query(
            "INSERT INTO codeharmony.lesson_plan(username,plan_name) VALUES ('user1',$1)",
            &[&payload.planName],
        )
        .await
        .map_err(|err| match err.as_db_error() {
            Some(err) => match *err.code() {
                SqlState::UNIQUE_VIOLATION => CodeHarmonyResponseError::BadRequest(
                    0,
                    "Plan already exists under this name".to_string(),
                ),
                _ => CodeHarmonyResponseError::DatabaseConnection,
            },
            None => CodeHarmonyResponseError::DatabaseConnection,
        })?;

    // Get the resulting name of the plan
    let inserted_plan_name: String = transaction
        .query(
            "SELECT plan_name FROM codeharmony.lesson_plan WHERE plan_name=$1",
            &[&payload.planName],
        )
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?[0]
        .get::<usize, String>(0);

    // Commit transaction, everything went well
    transaction
        .commit()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return ok with the new plan name
    Ok(HttpResponse::Ok().json(NewPlanResponse {
        planName: inserted_plan_name,
        msg: String::new(),
    }))
}

// Get list of plans for dashboard
#[get("/plan/list")]
async fn get_plan_list(
    db_pool: web::Data<Pool>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of plans from db
    let plan_list: Vec<Row> = client
        .query(
            "SELECT plan_name FROM codeharmony.lesson_plan WHERE username='user1'",
            &[],
        )
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the list of sessions from db
    let session_list: Vec<Row> = client
        .query(
            "SELECT plan_name,session_name FROM codeharmony.lesson_session WHERE username='user1'",
            &[],
        )
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Return list of plans
    Ok(HttpResponse::Ok().json(json!({
        "plans":plan_list.iter().map(|x| PlanInfoListItem{planName:x.get(0)}).collect::<Vec<PlanInfoListItem>>(),
        "sessions":session_list.iter().map(|row| SessionListItem{planName:row.get(0),sessionName:row.get(1)}).collect::<Vec<SessionListItem>>()
    })))
}

// Get all associated information about a plan
#[get("/plan/info/{plan_name}")]
async fn get_plan_info(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Expected plan name in uri".to_string(),
            ))
        }
    };

    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get sections from database
    let sections = get_plan_info_query(&client, plan_name).await?;

    // Return list of plans
    Ok(HttpResponse::Ok().json(sections))
}

// Get sections from the database
pub async fn get_plan_info_query(
    client: &Object,
    plan_name: &str,
) -> Result<Vec<PlanSection>, CodeHarmonyResponseError> {
    // Get rows from database
    let rows = client.query("SELECT section_name, section_type, section_elements, coding_data, order_pos FROM codeharmony.lesson_plan_section WHERE plan_name=$1 and username='user1' ORDER BY order_pos ASC",&[&plan_name]).await
                                      .map_err(|_| CodeHarmonyResponseError::InternalError(1,"Couldn't get rows from database".to_string()))?;

    // Convert rows to Vec
    let plan_sections = rows
        .iter()
        .map(PlanSection::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            println!("{:?}", e);
            CodeHarmonyResponseError::InternalError(2, "Invalid row format!".to_string())
        })?;

    Ok(plan_sections)
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct RenameSectionJSON {
    old_section_name: String,
    new_section_name: String,
}

//Rename a plan section
#[put("/plan/info/{plan_name}/rename")]
async fn update_plan_section_name(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
    section_names: web::Json<RenameSectionJSON>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Expected plan name in uri".to_string(),
            ))
        }
    };

    client.query("UPDATE codeharmony.lesson_plan_section SET section_name = $1 WHERE section_name = $2 and plan_name = $3 and username = $4",
    &[&section_names.new_section_name,&section_names.old_section_name, &plan_name, &"user1".to_string()]).await
        .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(1,"Couldn't update database".to_string())})?;

    // Return Ok!
    Ok(HttpResponse::Ok())
}

// Update the json data for a plan section
#[put("/plan/info/{plan_name}")]
async fn set_plan_section(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
    section: web::Json<PlanSection>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Expected plan name in uri".to_string(),
            ))
        }
    };

    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get the body as a json string
    let section_json = serde_json::to_value(&section.elements).map_err(|_| {
        CodeHarmonyResponseError::BadRequest(1, "Couldn't serialise elements".to_string())
    })?;

    let coding_data = serde_json::to_value(&section.codingData).map_err(|_| {
        CodeHarmonyResponseError::BadRequest(1, "Couldn't serialise coding data".to_string())
    })?;

    // Send the update query with the new section json data
    client.query("UPDATE codeharmony.lesson_plan_section SET section_elements = $1, order_pos = $2, coding_data = $5 WHERE plan_name = $3 and section_name=$4 and username='user1'",&[&section_json,&section.orderPos,&plan_name,&section.name,&coding_data]).await
          .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(1,"Couldn't update database".to_string())})?;

    // Return Ok!
    Ok(HttpResponse::Ok())
}

// Add new section to plan
#[post("/plan/info/{plan_name}")]
async fn perform_plan_operation(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
    operation: web::Json<PlanOperation>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Expected plan name in uri".to_string(),
            ))
        }
    };

    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    if &operation.request == "new-section" {
        let data: NewSectionData =
            serde_json::from_value(operation.data.to_owned()).map_err(|_| {
                CodeHarmonyResponseError::BadRequest(1, "Invalid operation data".to_string())
            })?;

        client.query("INSERT INTO codeharmony.lesson_plan_section(plan_name,username,order_pos,section_name,section_type) VALUES($1,$2,$3,$4,$5)",&[&plan_name,&"user1",&data.orderPos,&data.sectionName,&"LECTURE"]).await
              .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(2,"Couldn't add section".to_string())})?;
    }

    Ok(HttpResponse::Ok())
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct UpdateTypeJSON {
    section_name: String,
    new_type: String,
}

// Update section type
#[put("/plan/info/{plan_name}/update-type")]
async fn update_plan_section_type(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
    update_details: web::Json<UpdateTypeJSON>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get db client
    let client = db_pool
        .get()
        .await
        .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

    // Get plan name from uri
    let plan_name = match req.match_info().get("plan_name") {
        Some(plan_name) => plan_name,
        None => {
            return Err(CodeHarmonyResponseError::BadRequest(
                0,
                "Expected plan name in uri".to_string(),
            ))
        }
    };

    let new_type = if update_details.new_type.starts_with('L') {
        "LECTURE "
    } else {
        "CODING  "
    };

    client.query("UPDATE codeharmony.lesson_plan_section SET section_type = $1 WHERE section_name = $2 and plan_name = $3 and username = $4",
    &[&new_type, &update_details.section_name, &plan_name, &"user1"]).await
        .map_err(|e| {println!("{:?}",e);CodeHarmonyResponseError::InternalError(1,"Couldn't update database".to_string())})?;

    // Return Ok!
    Ok(HttpResponse::Ok())
}
