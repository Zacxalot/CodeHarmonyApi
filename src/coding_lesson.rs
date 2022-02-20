use crate::jsx_element::{ElementType, JSXChild, JSXElement};
use actix_web::{get, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct CodingLesson {
    lesson_id: u32,
    elements: Vec<JSXElement>,
}

#[get("/lesson/coding")]
async fn get_coding_lesson() -> impl Responder {
    HttpResponse::Ok().json(CodingLesson {
        lesson_id: 1,
        elements: vec![JSXElement {
            elType: ElementType::from_string("h1").unwrap(),
            props: json!({}),
            children: JSXChild::JSX(vec![]),
        }],
    })
}
