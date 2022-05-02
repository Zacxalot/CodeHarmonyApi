use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{delete, get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{endpoints::lesson_plan::PlanSection, utils::error::CodeHarmonyResponseError};

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(publish_plan)
        .service(search_plans)
        .service(get_plans)
        .service(delete_plan)
        .service(get_plan)
        .service(save_plan);
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct PublishData {
    planName: String,
    publishName: String,
    description: String,
}

//Publish a plan
#[post("/plan/publish")]
async fn publish_plan(
    db_pool: web::Data<deadpool_postgres::Pool>,
    publish_data: web::Json<PublishData>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let mut client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Insert parent record in published_lesson_plan
        const PARENT_STATEMENT: &str = "
        INSERT INTO codeharmony.published_lesson_plan(plan_name, username, description)
        VALUES ($1, $2, $3)
        ON CONFLICT ON CONSTRAINT published_lesson_plan_pk DO UPDATE SET description = $3;
        ";

        // Delete old records from published_lesson_plan_section
        const DELETE_STATEMENT: &str = "
        DELETE FROM codeharmony.published_lesson_plan_section *
        WHERE plan_name = $1 AND username = $2;
        ";

        // Copy over all of the plan sections
        const SECTIONS_STATEMENT:&str = "
        INSERT INTO codeharmony.published_lesson_plan_section(plan_name, username, section_elements, order_pos, coding_data, section_name, section_type)
        SELECT $1, $2::VARCHAR, section_elements, order_pos, coding_data, section_name, section_type
        FROM codeharmony.lesson_plan_section
        WHERE plan_name = $3 AND username = $2
        ";

        // Start transaction
        let transaction = client
            .transaction()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Parent execute
        transaction
            .query(
                PARENT_STATEMENT,
                &[
                    &publish_data.publishName,
                    &username,
                    &publish_data.description,
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        // Delete execute
        transaction
            .query(DELETE_STATEMENT, &[&publish_data.publishName, &username])
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        // Sections execute
        transaction
            .query(
                SECTIONS_STATEMENT,
                &[&publish_data.publishName, &username, &publish_data.planName],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        // Commit transaction, everything went well
        transaction
            .commit()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Return Ok!
        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[derive(Deserialize)]
struct SearchQuery {
    s: Option<String>,
}

#[derive(pg_mapper::TryFromRow, Deserialize, Serialize)]
struct SearchResult {
    plan_name: String,
    username: String,
    description: String,
}

#[get("/plan/search")]
async fn search_plans(
    query: web::Query<SearchQuery>,
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(_username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Different statements depending on if the query is empty
        const SEARCH_STATEMENT: &str = "SELECT plan_name, username, description FROM codeharmony.published_lesson_plan plp WHERE (to_tsvector(plan_name) || to_tsvector(username) || to_tsvector(description))  @@ websearch_to_tsquery($1)";
        const ALL_STATEMENT: &str =
            "SELECT plan_name, username, description FROM codeharmony.published_lesson_plan";

        let search = &query.into_inner().s.unwrap_or_default();

        // Execute search query
        let rows = {
            if search.is_empty() {
                client.query(ALL_STATEMENT, &[]).await.map_err(|e| {
                    println!("{:?}", e);
                    CodeHarmonyResponseError::InternalError(
                        0,
                        "Couldn't complete search".to_string(),
                    )
                })?
            } else {
                client
                    .query(SEARCH_STATEMENT, &[search])
                    .await
                    .map_err(|e| {
                        println!("{:?}", e);
                        CodeHarmonyResponseError::InternalError(
                            0,
                            "Couldn't complete search".to_string(),
                        )
                    })?
            }
        };

        // Convert rows into a Vec of SearchResults
        let results = rows
            .into_iter()
            .map(SearchResult::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(0, "Invalid rows".to_string())
            })?;

        // Return Ok with results!
        return Ok(HttpResponse::Ok().json(json!(results)));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[get("/plan/published")]
async fn get_plans(
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const ALL_STATEMENT: &str =
            "SELECT plan_name, username, description FROM codeharmony.published_lesson_plan WHERE username = $1";

        let rows = client
            .query(ALL_STATEMENT, &[&username])
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(0, "Couldn't complete search".to_string())
            })?;

        // Convert rows into a Vec of SearchResults
        let results = rows
            .into_iter()
            .map(SearchResult::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                println!("{:?}", e);
                CodeHarmonyResponseError::InternalError(0, "Invalid rows".to_string())
            })?;

        return Ok(HttpResponse::Ok().json(json!(results)));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[delete("/plan/published/{plan_name}")]
async fn delete_plan(
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
    path: web::Path<String>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    let plan_name = path.into_inner();

    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        const STATEMENT: &str =
            "DELETE FROM codeharmony.published_lesson_plan WHERE username=$1 AND plan_name=$2";

        client
            .query(STATEMENT, &[&username, &plan_name])
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        return Ok(HttpResponse::Ok());
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[derive(Serialize)]
struct PublishedPlan {
    plan_sections: Vec<PlanSection>,
    description: String,
}

#[get("/plan/published/{plan_name}/{plan_owner}")]
async fn get_plan(
    db_pool: web::Data<deadpool_postgres::Pool>,
    session: Session,
    path: web::Path<(String, String)>,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    let (plan_name, plan_owner) = path.into_inner();

    // Get username
    if let Ok(Some(_username)) = session.get::<String>("username") {
        // Get db client
        let client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Get rows from database
        const STATEMENT:&str = "SELECT section_name, section_type, section_elements, coding_data, order_pos FROM codeharmony.published_lesson_plan_section WHERE plan_name=$1 and username=$2 ORDER BY order_pos ASC";
        let rows = client
            .query(STATEMENT, &[&plan_name, &plan_owner])
            .await
            .map_err(|_| {
                CodeHarmonyResponseError::InternalError(
                    1,
                    "Couldn't get rows from database".to_string(),
                )
            })?;

        let plan_sections = rows
            .iter()
            .map(PlanSection::try_from)
            .collect::<Result<Vec<PlanSection>, _>>()
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::CouldntParseRows
            })?;

        const DESCRIPTION:&str = "SELECT description FROM codeharmony.published_lesson_plan WHERE plan_name=$1 and username = $2";
        let rows = client
            .query(DESCRIPTION, &[&plan_name, &plan_owner])
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::InternalError(
                    1,
                    "Couldn't get rows from database".to_string(),
                )
            })?;

        if let Some(row) = rows.get(0) {
            if let Ok(description) = row.try_get::<usize, String>(0) {
                return Ok(HttpResponse::Ok().json(PublishedPlan {
                    plan_sections,
                    description,
                }));
            }
        }

        return Ok(HttpResponse::Ok().json(PublishedPlan {
            plan_sections,
            description: String::new(),
        }));
    }
    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SaveData {
    planName: String,
    publishedName: String,
    publishHost: String,
}

//Save a published plan
#[post("/plan/save")]
async fn save_plan(
    db_pool: web::Data<deadpool_postgres::Pool>,
    save_data: web::Json<SaveData>,
    session: Session,
) -> Result<impl Responder, CodeHarmonyResponseError> {
    // Get username
    if let Ok(Some(username)) = session.get::<String>("username") {
        // Get db client
        let mut client = db_pool
            .get()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Insert parent record in published_lesson_plan
        const PARENT_STATEMENT: &str = "
        INSERT INTO codeharmony.lesson_plan(plan_name, username)
        VALUES ($1, $2)
        ";

        // Copy over all of the plan sections
        const SECTIONS_STATEMENT:&str = "
        INSERT INTO codeharmony.lesson_plan_section(plan_name, username, section_elements, order_pos, coding_data, section_name, section_type)
        SELECT $1, $2::VARCHAR, section_elements, order_pos, coding_data, section_name, section_type
        FROM codeharmony.published_lesson_plan_section
        WHERE plan_name = $3 AND username = $4
        ";

        // Start transaction
        let transaction = client
            .transaction()
            .await
            .map_err(|_| CodeHarmonyResponseError::DatabaseConnection)?;

        // Parent execute
        transaction
            .query(PARENT_STATEMENT, &[&save_data.planName, &username])
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        // Sections execute
        transaction
            .query(
                SECTIONS_STATEMENT,
                &[
                    &save_data.planName,
                    &username,
                    &save_data.publishedName,
                    &save_data.publishHost,
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                CodeHarmonyResponseError::DatabaseQueryFailed
            })?;

        // Commit transaction, everything went well
        transaction.commit().await.map_err(|e| {
            eprintln!("{:?}", e);
            CodeHarmonyResponseError::DatabaseQueryFailed
        })?;

        // Return Ok!
        return Ok(HttpResponse::Ok());
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}
