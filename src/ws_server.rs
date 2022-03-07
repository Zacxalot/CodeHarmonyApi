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
        },
        &req,
        stream,
    )
}

pub struct SessionRoom {
    student_addresses: HashSet<Recipient<Msg>>,
}

impl SessionRoom {
    fn new() -> Self {
        Self {
            student_addresses: HashSet::new(),
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SessionIdentifier {
    plan_name: String,
    session_name: String,
    host: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Msg(pub String);

// Connect to server
// Join session room
#[derive(Message, Debug)]
#[rtype(result = "()")]
struct Connect {
    addr: Recipient<Msg>,
}

// Join session room
#[derive(Message, Debug)]
#[rtype(result = "()")]
struct Join {
    identifier: SessionIdentifier,
    addr: Recipient<Msg>,
}

impl Handler<Join> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: Join, ctx: &mut Self::Context) -> Self::Result {
        // println!("Join heard - {:?}", msg);
        // match self.sessions.get_mut(&msg.identifier) {
        //     Some(room) => {
        //         room.student_addresses.insert(msg.addr);
        //     }
        //     None => {
        //         self.sessions.insert(msg.identifier, SessionRoom::new());
        //         self.sessions
        //             .get_mut(&msg.identifier)
        //             .unwrap()
        //             .student_addresses
        //             .insert(msg.addr);
        //     }
        // }
        // let room_option = self.sessions.get_mut(&msg.identifier);
    }
}
