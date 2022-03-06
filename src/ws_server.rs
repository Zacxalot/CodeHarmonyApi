use std::collections::HashMap;

use actix::{clock::Instant, Actor, ActorContext, Addr, AsyncContext, Context, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

pub async fn session_service(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(WsClientSession {}, &req, stream)
}

struct WsClientSession {}

impl Actor for WsClientSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Hi world");
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
            // ws::Message::Ping(_) => self.heartbeat = Instant::now(),
            ws::Message::Text(text) => {
                let split: Vec<&str> = text.splitn(2, ' ').collect();
                if split.len() == 2 {
                    match split[0] {
                        "sPos" => {
                            println!("Setting pos")
                        }
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

pub struct SessionServer {
    sessions: HashMap<usize, usize>,
}

impl SessionServer {
    pub fn new() -> SessionServer {
        let mut sessions: HashMap<usize, usize> = HashMap::new();
        SessionServer { sessions: sessions }
    }
}

impl Actor for SessionServer {
    type Context = Context<Self>;
}
