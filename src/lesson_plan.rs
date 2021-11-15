use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use deadpool_postgres::{Pool};
use serde::{Serialize};


#[derive(Serialize)]
struct NewPlanResponse {
    plan_name:String,
    msg:String
}

// Request a new plan
#[post("/plan/new")]
async fn create_lesson_plan(db_pool: web::Data<Pool>, req: HttpRequest) -> impl Responder {

    // Get db client
    let client = match db_pool.get().await{
        Ok(x) => x,
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-1".to_string(),msg:"Couldn't connect to server".to_string()})
    };


    // Get the plan name from the header
    let plan_name = match req.headers().get("plan_name") {
        Some(x) => match x.to_str(){
            Ok(x) => x,
            Err(_) => return HttpResponse::BadRequest().json(NewPlanResponse {
                plan_name:"-3".to_string(),msg:"Invalid plan name".to_string()
            })
        },
        None => return HttpResponse::BadRequest().json(NewPlanResponse {
            plan_name:"-3".to_string(),msg:"Plan name already exists".to_string()
        })
    };


    // If there's already a plan with that name, return an error
    match client.query(&format!("SELECT plan_name, username FROM codeharmony.lesson_plan WHERE username='user1' AND plan_name='{}';",plan_name),&[]).await{
        Ok(x) => if x.len() > 0{
            return HttpResponse::BadRequest().json(NewPlanResponse {plan_name:"-2".to_string(),msg:"Plan name already exists".to_string()})
        },
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-2".to_string(),msg:"Error accessing database".to_string()})
    };

    // Try to the new plan to the db
    match client.query(&format!("INSERT INTO codeharmony.lesson_plan(username,plan_name) VALUES ('user1','{}');",plan_name),&[]).await{
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().json(NewPlanResponse {plan_name:"-2".to_string(),msg:"Error creating new lesson plan".to_string()})
    };


    HttpResponse::Ok().json(NewPlanResponse {
        plan_name:plan_name.to_string(),
        msg:String::new()
    })
}