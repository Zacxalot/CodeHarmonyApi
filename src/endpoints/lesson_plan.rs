use crate::utils::error::CodeHarmonyResponseError;
use crate::utils::jsx_element::JSXElement;
use actix_session::Session;
use actix_web::{get, post, put, web, HttpRequest, HttpResponse, Responder};
use deadpool_postgres::{Object, Pool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::TryFrom;
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
        .service(get_plan_and_session_list)
        .service(get_plan_info)
        .service(set_plan_section)
        .service(perform_plan_operation)
        .service(update_plan_section_name)
        .service(update_plan_section_type);
}

impl TryFrom<&tokio_postgres::Row> for PlanSection {
    type Error = Box<dyn std::error::Error>;
    fn try_from(row: &tokio_postgres::Row) -> Result<Self, Self::Error> {
        #[allow(non_snake_case)]
        if let (Ok(name), Ok(sectionType), Ok(elements), Ok(codingData), Ok(orderPos)) = (
            row.try_get::<&str, String>("section_name"),
            row.try_get::<&str, String>("section_type"),
            row.try_get::<&str, serde_json::Value>("section_elements"),
            row.try_get::<&str, serde_json::Value>("coding_data"),
            row.try_get::<&str, i16>("order_pos"),
        ) {
            return Ok(PlanSection {
                name,
                sectionType,
                elements: serde_json::from_value(elements)?,
                codingData: serde_json::from_value(codingData)?,
                orderPos,
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
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
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
                "INSERT INTO codeharmony.lesson_plan(username,plan_name) VALUES ($1,$2)",
                &[&username, &payload.planName],
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
                "SELECT plan_name FROM codeharmony.lesson_plan WHERE plan_name=$1 and username=$2",
                &[&payload.planName, &username],
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
        return Ok(HttpResponse::Ok().json(NewPlanResponse {
            planName: inserted_plan_name,
            msg: String::new(),
        }));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Get list of plans and sessions for dashboard
#[get("/plan/list")]
async fn get_plan_and_session_list(
    db_pool: web::Data<Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get the list of plans from db
        let plan_list: Vec<Row> = client
            .query(
                "SELECT plan_name FROM codeharmony.lesson_plan WHERE username=$1",
                &[&username],
            )
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get the list of sessions from db
        let session_list: Vec<Row> = client
            .query(
                "SELECT plan_name,session_name FROM codeharmony.lesson_session WHERE username=$1",
                &[&username],
            )
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Return list of plans and sessions
        return Ok(HttpResponse::Ok().json(json!({
            "plans":plan_list.iter().map(|x| PlanInfoListItem{planName:x.get(0)}).collect::<Vec<PlanInfoListItem>>(),
            "sessions":session_list.iter().map(|row| SessionListItem{planName:row.get(0),sessionName:row.get(1)}).collect::<Vec<SessionListItem>>()
        })));
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Get all associated information about a plan
#[get("/plan/info/{plan_name}")]
async fn get_plan_info(
    db_pool: web::Data<Pool>,
    path: web::Path<String>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get plan name from uri
        let plan_name = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get sections from database
        let sections = get_plan_info_query(&client, &plan_name, &username).await?;

        // Return list of plans
        return Ok(HttpResponse::Ok().json(sections));
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Get sections from the database
pub async fn get_plan_info_query(
    client: &Object,
    plan_name: &str,
    username: &str,
) -> Result<Vec<PlanSection>, CodeHarmonyResponseError> {
    // Get rows from database
    const STATEMENT:&str = "SELECT section_name, section_type, section_elements, coding_data, order_pos FROM codeharmony.lesson_plan_section WHERE plan_name=$1 and username=$2 ORDER BY order_pos ASC";
    let rows = client
        .query(STATEMENT, &[&plan_name, &username])
        .await
        .map_err(|_| {
            CodeHarmonyResponseError::InternalError(
                1,
                "Couldn't get rows from database".to_string(),
            )
        })?;

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
    path: web::Path<String>,
    section_names: web::Json<RenameSectionJSON>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get plan name from uri
        let plan_name = path.into_inner();

        const STATEMENT:&str = "UPDATE codeharmony.lesson_plan_section SET section_name = $1 WHERE section_name = $2 and plan_name = $3 and username = $4";

        client
            .query(
                STATEMENT,
                &[
                    &section_names.new_section_name,
                    &section_names.old_section_name,
                    &plan_name,
                    &username,
                ],
            )
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(1, "Couldn't update database".to_string())
            })?;

        // Return Ok!
        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Update the json data for a plan section
#[put("/plan/info/{plan_name}")]
async fn set_plan_section(
    db_pool: web::Data<Pool>,
    path: web::Path<String>,
    section: web::Json<PlanSection>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get plan name from uri
        let plan_name = path.into_inner();

        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get the json values from body
        let section_json = serde_json::to_value(&section.elements).map_err(|_| {
            CodeHarmonyResponseError::BadRequest(0, "Couldn't deserialise elements".to_string())
        })?;

        let coding_data = serde_json::to_value(&section.codingData).map_err(|_| {
            CodeHarmonyResponseError::BadRequest(1, "Couldn't deserialise coding data".to_string())
        })?;

        // Check format of values math structs
        let _section_json_formatted: Vec<JSXElement> = serde_json::from_value(section_json.clone())
            .map_err(|_| {
                CodeHarmonyResponseError::BadRequest(2, "Invalid elements format".to_string())
            })?;

        let _coding_data_formatted: CodingData = serde_json::from_value(section_json.clone())
            .map_err(|_| {
                CodeHarmonyResponseError::BadRequest(2, "Invalid coding data format".to_string())
            })?;

        // Send the update query with the new section json data
        const STATEMENT:&str = "UPDATE codeharmony.lesson_plan_section SET section_elements = $1, order_pos = $2, coding_data = $5 WHERE plan_name = $3 and section_name=$4 and username=$6";

        client
            .query(
                STATEMENT,
                &[
                    &section_json,
                    &section.orderPos,
                    &plan_name,
                    &section.name,
                    &coding_data,
                    &username,
                ],
            )
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(4, "Couldn't update database".to_string())
            })?;

        // Return Ok!
        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

// Perform plan operation
#[post("/plan/info/{plan_name}")]
async fn perform_plan_operation(
    db_pool: web::Data<Pool>,
    req: HttpRequest,
    operation: web::Json<PlanOperation>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
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

            const STATEMENT:&str = "INSERT INTO codeharmony.lesson_plan_section(plan_name,username,order_pos,section_name,section_type) VALUES($1,$2,$3,$4,'LECTURE ')";

            client
                .query(
                    STATEMENT,
                    &[&plan_name, &username, &data.orderPos, &data.sectionName],
                )
                .await
                .map_err(|err| match err.as_db_error() {
                    Some(err) => match *err.code() {
                        SqlState::UNIQUE_VIOLATION => CodeHarmonyResponseError::BadRequest(
                            0,
                            "Name already in use".to_string(),
                        ),
                        SqlState::CHECK_VIOLATION => {
                            CodeHarmonyResponseError::BadRequest(1, "Too short".to_string())
                        }
                        _ => CodeHarmonyResponseError::DatabaseConnection,
                    },
                    None => CodeHarmonyResponseError::DatabaseConnection,
                })?;
        }

        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
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
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
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

        const STATEMENT:&str = "UPDATE codeharmony.lesson_plan_section SET section_type = $1 WHERE section_name = $2 and plan_name = $3 and username = $4";

        client
            .query(
                STATEMENT,
                &[
                    &new_type,
                    &update_details.section_name,
                    &plan_name,
                    &username,
                ],
            )
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(1, "Couldn't update database".to_string())
            })?;

        // Return Ok!
        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}
