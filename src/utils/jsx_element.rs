use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug)]
pub struct JSXElement {
    pub elType: ElementType,
    pub props: Value,
    pub children: JSXChild,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize, Debug)]
pub enum JSXChild {
    JSX(Vec<JSXElement>),
    String(String),
    Empty,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
pub enum ElementType {
    h1,
    p,
    img,
}

// impl ElementType {
//     pub fn from_string(string: &str) -> Result<ElementType, &str> {
//         match string {
//             "h1" => Ok(ElementType::h1),
//             "p" => Ok(ElementType::p),
//             "img" => Ok(ElementType::img),
//             _ => Err("Invalid element type"),
//         }
//     }
// }
