
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{Value};


#[derive(Serialize, Deserialize)]
struct CodingLesson {
    lesson_id:u32,
    elements:Vec<CodingLessonElement>
}

#[derive(Serialize, Deserialize)]
struct CodingLessonElement{
    el_type:ElementType,
    props:Vec<Value>,
    children:Vec<CodingLessonElement>
}



#[derive(Serialize, Deserialize)]
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



#[get("/lesson/coding")]
async fn get_coding_lesson() -> impl Responder {
    let a = CodingLessonElement{el_type:ElementType::from_string("div").unwrap(),props: vec![], children:vec![]};

    HttpResponse::Ok().json(CodingLesson {
        lesson_id:1,
        elements:vec![CodingLessonElement{el_type:ElementType::from_string("h1").unwrap(),props: vec![], children:vec![a]}]
    })
}