use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize,Debug)]
pub struct JSXElement{
    pub el_type:ElementType,
    pub props:Value,
    pub children:JSXChild
}

#[derive(Deserialize, Serialize,Debug)]
pub enum JSXChild{
    JSX(Vec<JSXElement>),
    String(String),
    Empty
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize,Debug)]
pub enum ElementType{
    h1,
    p,
    img
}

impl ElementType{
    pub fn from_string(string: &str) -> Result<ElementType,&str>{
        match string {
            "h1" => return Ok(ElementType::h1),
            "p" => return Ok(ElementType::p),
            "img" => return Ok(ElementType::img),
            _ => Err("Invalid element type")
        }
    }
}