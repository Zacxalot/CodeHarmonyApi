use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws;

use crate::actors::ws_server::{
    ControlInstruction, Leave, SendTextMessage, SessionIdentifier, SessionServer, SetStudentDoc,
    StudentJoin, TeacherJoin, UpdateStudentCode, WSResponse,
};

pub struct WsClientSession {
    pub addr: Addr<SessionServer>,
    pub connected_session: Option<SessionIdentifier>,
    pub username: String,
}

impl Actor for WsClientSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        println!("New Connection");
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        if let Some(connected_session) = &self.connected_session {
            self.addr.do_send(Leave {
                identifier: connected_session.clone(),
                addr: ctx.address().recipient::<WSResponse>(),
            });
        }

        println!("Disconnection");
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
                let split: Vec<&str> = text.splitn(2, ' ').collect();
                if split.len() == 2 {
                    let addr = ctx.address().recipient::<WSResponse>();
                    let username = self.username.clone();

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
                            username,
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
                            username,
                        })
                    } else if split[0] == "tInst" {
                        println!("{:?}", self.connected_session);
                        if let Some(identifier) = self.connected_session.as_ref() {
                            self.addr.do_send(ControlInstruction {
                                instruction: split[1].to_owned(),
                                identifier: identifier.clone(),
                            });
                        }
                    } else if split[0] == "sUpdate" {
                        if let Some(identifier) = self.connected_session.as_ref() {
                            self.addr.do_send(UpdateStudentCode {
                                identifier: identifier.clone(),
                                username,
                                code: split[1].to_owned(),
                                student_addr: addr,
                            });
                        }
                    } else if split[0] == "sDoc" {
                        if let Some(identifier) = self.connected_session.as_ref() {
                            self.addr.do_send(SetStudentDoc {
                                identifier: identifier.clone(),
                                username,
                                code: split[1].to_owned(),
                                student_addr: addr,
                            });
                        }
                    } else if split[0] == "txtm" {
                        if let Some(identifier) = self.connected_session.as_ref() {
                            self.addr.do_send(SendTextMessage {
                                identifier: identifier.clone(),
                                username,
                                text: split[1].to_owned(),
                            });
                        }
                    }
                }
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
            WSResponse::Close => ctx.close(None),
        }
    }
}
