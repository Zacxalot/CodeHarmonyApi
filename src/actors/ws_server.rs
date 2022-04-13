use std::collections::HashMap;

use crate::{actors::ws_session::WsClientSession, utils::error::CodeHarmonyResponseError};
use actix::{Actor, Addr, Context, Handler, Message, Recipient};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use bimap::BiMap;
use serde_json::json;
use uuid::Uuid;

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

#[derive(Debug)]
pub struct Student {
    addr: Recipient<WSResponse>,
}

#[allow(dead_code)]
pub struct SessionRoom {
    teacher: User,
    students: BiMap<String, Recipient<WSResponse>>,
    current_section: usize,
    current_student_username: Option<String>,
}

impl SessionRoom {
    fn new(teacher_addr: Recipient<WSResponse>, username: String) -> Self {
        Self {
            students: BiMap::new(),
            teacher: User {
                addr: teacher_addr,
                username,
            },
            current_section: 0,
            current_student_username: None,
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

            // Tell the student what section we're on
            msg.addr
                .do_send(WSResponse::Msg(format!("sec {}", session.current_section)));

            // Insert the student into the list
            session.students.insert(msg.username, msg.addr);
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
        if instruction[0] == "setSection" && instruction.len() == 2 {
            if let Ok(new_value) = instruction[1].parse::<usize>() {
                if let Some(session) = self.sessions.get_mut(&msg.identifier) {
                    session.current_section = new_value;
                    let msg = format!("sec {}", new_value);
                    println!("Student addresses {:?}", session.students);
                    for addr in session.students.right_values() {
                        addr.do_send(WSResponse::Msg(msg.to_owned()));
                        println!("Sent instruction");
                    }
                }
            }
        } else if instruction[0] == "subscribe" && instruction.len() == 2 {
            if let Some(session) = self.sessions.get_mut(&msg.identifier) {
                if let Some(student_addr) = session.students.get_by_left(instruction[1]) {
                    // Unsub the old student if there is one and it isn't the same student
                    if let Some(to_unsub_username) = &session.current_student_username {
                        if to_unsub_username != instruction[1] {
                            if let Some(to_unsub_addr) =
                                session.students.get_by_left(to_unsub_username)
                            {
                                to_unsub_addr.do_send(WSResponse::Msg("unsub".to_owned()));
                            }
                        }
                    }

                    session.current_student_username = Some(instruction[1].to_owned());
                    student_addr.do_send(WSResponse::Msg("subscribe".to_owned()));
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
        if let Some(session) = self.sessions.get(&msg.identifier) {
            if session.teacher.username == msg.username {
                return session
                    .students
                    .left_values()
                    .map(|username| username.to_owned())
                    .collect::<Vec<String>>();
            }
        }
        vec![]
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct UpdateStudentCode {
    pub identifier: SessionIdentifier,
    pub username: String,
    pub code: String,
    pub student_addr: Recipient<WSResponse>,
}

impl Handler<UpdateStudentCode> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: UpdateStudentCode, _: &mut Self::Context) -> Self::Result {
        // If the session exists and this is the student the room is looking at,
        // send the code to the teacher
        if let Some(session) = self.sessions.get(&msg.identifier) {
            if let Some(current_student_username) = &session.current_student_username {
                if current_student_username == &msg.username {
                    session
                        .teacher
                        .addr
                        .do_send(WSResponse::Msg(format!("sUpdate {}", msg.code)));
                    return;
                }
            }
        }

        // If not, tell the student we don't want to hear from them anymore
        msg.student_addr
            .do_send(WSResponse::Msg("unsub".to_owned()));
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SetStudentDoc {
    pub identifier: SessionIdentifier,
    pub username: String,
    pub code: String,
    pub student_addr: Recipient<WSResponse>,
}

impl Handler<SetStudentDoc> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: SetStudentDoc, _: &mut Self::Context) -> Self::Result {
        // If the session exists and this is the student the room is looking at,
        // send the starting code to the teacher
        if let Some(session) = self.sessions.get(&msg.identifier) {
            if let Some(current_student_username) = &session.current_student_username {
                if current_student_username == &msg.username {
                    session
                        .teacher
                        .addr
                        .do_send(WSResponse::Msg(format!("sDoc {}", msg.code)));
                    return;
                }
            }
        }

        // If not, tell the student we don't want to hear from them anymore
        msg.student_addr
            .do_send(WSResponse::Msg("unsub".to_owned()));
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SendTextMessage {
    pub identifier: SessionIdentifier,
    pub username: String,
    pub text: String,
}

impl Handler<SendTextMessage> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: SendTextMessage, _: &mut Self::Context) -> Self::Result {
        if let Some(session) = self.sessions.get_mut(&msg.identifier) {
            let response = json!({"username":msg.username, "uuid":Uuid::new_v4().to_string(), "text": msg.text});
            let msg = format!("txtm {}", response);

            println!("{}", response);

            // Send message to all students
            for addr in session.students.right_values() {
                addr.do_send(WSResponse::Msg(msg.to_owned()));
            }

            // And the teacher
            session.teacher.addr.do_send(WSResponse::Msg(msg));
        }
    }
}
