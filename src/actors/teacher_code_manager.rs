use actix::{Actor, Context, Handler, Message};
use bimap::BiMap;
use rand::Rng;

pub struct TeacherCodeManager {
    map: BiMap<String, String>,
}

impl Actor for TeacherCodeManager {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("Teacher code manager started");
    }
}

impl TeacherCodeManager {
    pub fn new() -> TeacherCodeManager {
        TeacherCodeManager { map: BiMap::new() }
    }
}

#[derive(Message)]
#[rtype(result = "String")]
pub struct GetCode {
    pub username: String,
}

impl Handler<GetCode> for TeacherCodeManager {
    type Result = String;

    fn handle(&mut self, msg: GetCode, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(code) = self.map.get_by_left(&msg.username) {
            // If there is already a code, return it
            code.into()
        } else {
            // If it doesn't, generate, insert and return it
            let mut rng = rand::thread_rng();
            let code = format!("{:0>6}", rng.gen_range(1..999999));
            self.map.insert(msg.username, code.clone());
            code
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<String>")]
pub struct GetTeacher {
    pub code: String,
}

impl Handler<GetTeacher> for TeacherCodeManager {
    type Result = Option<String>;

    fn handle(&mut self, msg: GetTeacher, _ctx: &mut Self::Context) -> Self::Result {
        // If code exists, return the teacher username
        if let Some(username) = self.map.get_by_right(&msg.code) {
            return Some(username.clone());
        }
        None
    }
}
