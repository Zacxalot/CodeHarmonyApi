use actix::{Actor, ActorContext, Addr, Handler, StreamHandler};
use actix_web_actors::ws;

use crate::ws_server::{Msg, SessionIdentifier, SessionServer};

pub struct WsClientSession {
    pub addr: Addr<SessionServer>,
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
        println!("Message");
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
                    // Handle text commands
                    match split[0] {
                        // "sPos" => {
                        //     println!("Setting pos")
                        // }
                        // "join" => {
                        //     let split_id: Vec<&str> = text.splitn(3, ':').collect();
                        //     if split_id.len() == 3 {
                        //         let addr = ctx.address().recipient::<Msg>();
                        //         self.addr.do_send(Join {
                        //             identifier: SessionIdentifier {
                        //                 plan_name: split_id[0].to_owned(),
                        //                 session_name: split_id[1].to_owned(),
                        //                 host: split_id[2].to_owned(),
                        //             },
                        //             addr: addr,
                        //         });
                        //     }

                        //     println!("Joining")
                        // }
                        _ => {}
                    }
                }

                println!("msg is tasty");
                println!("{}", text);
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
impl Handler<Msg> for WsClientSession {
    type Result = ();

    fn handle(&mut self, msg: Msg, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0);
    }
}
