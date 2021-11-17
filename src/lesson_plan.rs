use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, post, web};
use deadpool_postgres::{Pool};
use serde::{Deserialize, Serialize};
use tokio_postgres::error::DbError;


#[derive(Serialize)]
struct NewPlanResponse {
    plan_name:String,
    msg:String
}

#[derive(Deserialize)]
struct NewPlanRequest {
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