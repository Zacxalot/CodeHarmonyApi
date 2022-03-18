use std::collections::{HashMap, HashSet};

use actix::{Actor, Addr, Context, Handler, Message, Recipient};
use actix_session::Session;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use crate::{actors::ws_session::WsClientSession, utils::error::CodeHarmonyResponseError};

pub async fn session_service(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<SessionServer>>,
    session: Session,
) -> Result<HttpResponse, CodeHarmonyResponseError> {
    if let Ok(Some(username)) = session.get::<String>("username") {
        return ws::start(
            WsClientSession {
                addr: srv.get_ref().clone(),
                connected_session: None,
                username,
            },
            &req,
            stream,
        )
        .map_err(|_| {
            CodeHarmonyResponseError::InternalError(
                0,
                "Couldn't create websocket connection".into(),
            )
        });
    }

    Err(CodeHarmonyResponseError::NotLoggedIn)
}

#[derive(PartialEq, Hash, Eq, Debug)]
pub struct User {
    addr: Recipient<WSResponse>,
    username: String,
}

#[allow(dead_code)]
pub struct SessionRoom {
    teacher: User,
    students: HashSet<User>,
    current_section: usize,
}

impl SessionRoom {
    fn new(teacher_addr: Recipient<WSResponse>, username: String) -> Self {
        Self {
            students: HashSet::new(),
            teacher: User {
                addr: teacher_addr,
                username,
            },
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
    pub username: String,
}

impl Handler<TeacherJoin> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: TeacherJoin, _: &mut Self::Context) -> Self::Result {
        // If the room exists, reset the teacher values
        if let Some(room) = self.sessions.get_mut(&msg.identifier) {
            room.teacher = User {
                username: msg.username,
                addr: msg.addr.clone(),
            }
        } else {
            // If it doesn't create a new room with teacher details.
            self.sessions.insert(
                msg.identifier.clone(),
                SessionRoom::new(msg.addr.clone(), msg.username),
            );
        }

        // Set the active room for the teacher
        msg.addr
            .do_send(WSResponse::SetConnectedSession(msg.identifier));
        println!("Teacher started session")
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct StudentJoin {
    pub identifier: SessionIdentifier,
    pub addr: Recipient<WSResponse>,
    pub username: String,
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
            session.students.insert(User {
                addr: msg.addr,
                username: msg.username,
            });
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

        println!("Instruction is{:?}", instruction);

        // If setSection, get new value and session room
        // Set current_section val and send the msg to students in room
        if instruction.len() == 2 && instruction[0] == "setSection" {
            if let Ok(new_value) = instruction[1].parse::<usize>() {
                if let Some(session) = self.sessions.get_mut(&msg.identifier) {
                    session.current_section = new_value;
                    let msg = format!("sec {}", new_value);
                    println!("Student addresses {:?}", session.students);
                    for student in session.students.iter() {
                        student.addr.do_send(WSResponse::Msg(msg.to_owned()));
                        println!("Sent instruction");
                    }
                }
            }
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<String>")]
pub struct GetStudentData {
    pub identifier: SessionIdentifier,
    pub username: String,
}

impl Handler<GetStudentData> for SessionServer {
    type Result = Vec<String>;

    fn handle(&mut self, msg: GetStudentData, _: &mut Self::Context) -> Self::Result {
        if let Some(session) = self.sessions.get_mut(&msg.identifier) {
            if session.teacher.username == msg.username {
                return session
                    .students
                    .iter()
                    .map(|user| user.username.clone())
                    .collect::<Vec<String>>();
            }
        }
        vec![]
    }
}
