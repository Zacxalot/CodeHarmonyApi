
use actix_web::{get, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::jsx_element::{JSXElement, ElementType, JSXChild};

#[derive(Serialize, Deserialize)]
struct CodingLesson {
    lesson_id:u32,
    elements:Vec<JSXElement>
}

#[get("/lesson/coding")]
async fn get_coding_lesson() -> impl Responder {
    HttpResponse::Ok().json(CodingLesson {
        lesson_id:1,
        elements:vec![JSXElement{el_type:ElementType::from_string("h1").unwrap(),props: vec![], children:JSXChild::JSX(vec![])}]
    })
}