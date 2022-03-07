use std::collections::{HashMap, HashSet};

use actix::{
    clock::Instant, Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Recipient,
    StreamHandler,
};
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
        let mut sessions: HashMap<SessionIdentifier, SessionRoom> = HashMap::new();
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

    fn handle(&mut self, msg: TeacherJoin, ctx: &mut Self::Context) -> Self::Result {
        if let std::collections::hash_map::Entry::Vacant(e) = self.sessions.entry(msg.identifier) {
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

    fn handle(&mut self, msg: StudentJoin, ctx: &mut Self::Context) -> Self::Result {
        println!("Printing sessions");
        for (session, room) in self.sessions.iter() {
            println!("---{:?}", session);
        }

        println!("{:?}", &msg.identifier);

        if let Some(session) = self.sessions.get_mut(&msg.identifier) {
            msg.addr
                .do_send(WSResponse::SetConnectedSession(msg.identifier));
            println!("Student joined session");
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

    fn handle(&mut self, msg: ControlInstruction, ctx: &mut Self::Context) -> Self::Result {
        let instruction: Vec<&str> = msg.instruction.split(' ').collect();
        println!("SPLIT INSTRUCTION");
        #[allow(clippy::collapsible_if)]
        if !instruction.is_empty() {
            println!("INSTRUCTION NOT EMPTY");
            if instruction[0] == "setSection" && instruction.len() == 2 {
                println!("SETTING SECTION");
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
        }

        println!("Student joined session");
    }
}
