use actix_web::{HttpResponse, Responder, post,get};
use serde::{Serialize,Deserialize};


#[derive(Serialize, Deserialize)]
struct NewPlanResponse {
    id:i32
}

#[post("/plan/new")]
async fn create_lesson_plan() -> impl Responder {
    
    HttpResponse::Ok().json(NewPlanResponse {
        id:1
    })
}