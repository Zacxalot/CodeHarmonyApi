use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::utils::error::CodeHarmonyResponseError;

// Group all of the services together into a single init
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(publish_plan).service(search_plans);
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct PublishData {
    planName: String,
    publishName: String,
    description: String,
}

//Rename a plan section
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
