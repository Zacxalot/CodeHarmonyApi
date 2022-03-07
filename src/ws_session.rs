use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws;

use crate::ws_server::{
    ControlInstruction, SessionIdentifier, SessionServer, StudentJoin, TeacherJoin, WSResponse,
};

pub struct WsClientSession {
    pub addr: Addr<SessionServer>,
    pub connected_session: Option<SessionIdentifier>,
}

impl Actor for WsClientSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("New Connection");
    }
}

// Handles messages from clients
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClientSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match item {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(item) => item,
        };

        match msg {
            ws::Message::Text(text) => {
                println!("{:?}", &text);
                let split: Vec<&str> = text.splitn(2, ' ').collect();
                if split.len() == 2 {
                    let addr = ctx.address().recipient::<WSResponse>();

                    // Handle text commands
                    if split[0] == "tJoin" {
                        let split_id: Vec<&str> = split[1].splitn(3, ':').collect();

                        self.addr.do_send(TeacherJoin {
                            identifier: SessionIdentifier {
                                plan_name: split_id[0].to_owned(),
                                session_name: split_id[1].to_owned(),
                                host: split_id[2].to_owned(),
                            },
                            addr,
                        })
                    } else if split[0] == "sJoin" {
                        let split_id: Vec<&str> = split[1].splitn(3, ':').collect();
                        self.addr.do_send(StudentJoin {
                            identifier: SessionIdentifier {
                                plan_name: split_id[0].to_owned(),
                                session_name: split_id[1].to_owned(),
                                host: split_id[2].to_owned(),
                            },
                            addr,
                        })
                    } else if split[0] == "tInst" {
                        println!("{:?}", self.connected_session);
                        if let Some(identifier) = self.connected_session.as_ref() {
                            self.addr.do_send(ControlInstruction {
                                instruction: split[1].to_owned(),
                                identifier: identifier.clone(),
                            });
                        }
                    }
                }
                println!("GOT - {}", text);
            }
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            _ => ctx.text("What do you call this?"),
        }
    }
}

// Messages from sever to client
impl Handler<WSResponse> for WsClientSession {
    type Result = ();
    fn handle(&mut self, response: WSResponse, ctx: &mut Self::Context) -> Self::Result {
        match response {
            WSResponse::Msg(message) => ctx.text(message),
            WSResponse::SetConnectedSession(identifier) => {
                println!("SETTING CONNECTED SESSION");
                self.connected_session = Some(identifier)
            }
        }
    }
}
