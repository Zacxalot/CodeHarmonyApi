use std::collections::{HashMap, HashSet};

use actix::{Actor, Addr, Context, Handler, Message, Recipient};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use crate::ws_session::WsClientSession;

pub async fn session_service(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<SessionServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsClientSession {
            addr: srv.get_ref().clone(),
            connected_session: None,
        },
        &req,
        stream,
    )
}

#[allow(dead_code)]
pub struct SessionRoom {
    teacher_address: Recipient<WSResponse>,
    student_addresses: HashSet<Recipient<WSResponse>>,
    current_section: usize,
}

impl SessionRoom {
    fn new(teacher_addr: Recipient<WSResponse>) -> Self {
        Self {
            student_addresses: HashSet::new(),
            teacher_address: teacher_addr,
            current_section: 0,
        }
    }
}

pub struct SessionServer {
    sessions: HashMap<SessionIdentifier, SessionRoom>,
}

impl SessionServer {
    pub fn new() -> SessionServer {
        let sessions: HashMap<SessionIdentifier, SessionRoom> = HashMap::new();
        SessionServer { sessions }
    }
}

impl Actor for SessionServer {
    type Context = Context<Self>;
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SessionIdentifier {
    pub plan_name: String,
    pub session_name: String,
    pub host: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum WSResponse {
    Msg(String),
    SetConnectedSession(SessionIdentifier),
}

// Teacher Join
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct TeacherJoin {
    pub identifier: SessionIdentifier,
    pub addr: Recipient<WSResponse>,
}

impl Handler<TeacherJoin> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: TeacherJoin, _: &mut Self::Context) -> Self::Result {
        if let std::collections::hash_map::Entry::Vacant(e) =
            self.sessions.entry(msg.identifier.clone())
        {
            // Set the room address in the teacher connection
            msg.addr
                .do_send(WSResponse::SetConnectedSession(msg.identifier));

            // Set to room owner
            e.insert(SessionRoom::new(msg.addr));
        }
        println!("Teacher started session")
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct StudentJoin {
    pub identifier: SessionIdentifier,
    pub addr: Recipient<WSResponse>,
}

impl Handler<StudentJoin> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: StudentJoin, _: &mut Self::Context) -> Self::Result {
        // If the room exists,
        if let Some(session) = self.sessions.get_mut(&msg.identifier) {
            // set the address in the students connection
            msg.addr
                .do_send(WSResponse::SetConnectedSession(msg.identifier));
            println!("Student joined session");
            // And insert the student into the list
            session.student_addresses.insert(msg.addr);
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ControlInstruction {
    pub instruction: String,
    pub identifier: SessionIdentifier,
}

impl Handler<ControlInstruction> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: ControlInstruction, _: &mut Self::Context) -> Self::Result {
        let instruction: Vec<&str> = msg.instruction.split(' ').collect();

        // If setSection, get new value and session room
        // Set current_section val and send the msg to students in room
        if instruction.len() == 2 && instruction[0] == "setSection" {
            if let Ok(new_value) = instruction[1].parse::<usize>() {
                if let Some(session) = self.sessions.get_mut(&msg.identifier) {
                    session.current_section = new_value;
                    let msg = format!("sec {}", new_value);
                    for student in session.student_addresses.iter() {
                        student.do_send(WSResponse::Msg(msg.to_owned()));
                        println!("Sent instruction");
                    }
                }
            }
        }

        println!("Student joined session");
    }
}
