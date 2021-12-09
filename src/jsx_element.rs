use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize,Debug)]
pub struct JSXElement{
    pub el_type:ElementType,
    pub props:Vec<Value>,
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
    div,
    h1,
    p
}

impl ElementType{
    pub fn from_string(string: &str) -> Result<ElementType,&str>{
        match string {
            "div" => return Ok(ElementType::div),
            "h1" => return Ok(ElementType::h1),
            "p" => return Ok(ElementType::p),
            _ => Err("Invalid element type")
        }
    }
}