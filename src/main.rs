use actix::Actor;
use actix_web::{
    get,
    web::{self},
    App, HttpResponse, HttpServer, Responder,
};
use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

mod coding_lesson;
mod error;
mod jsx_element;
mod lesson_plan;
mod lesson_session;
mod ws_server;
mod ws_session;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup Postgres pool
    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some("postgres".to_string());
    cfg.user = Some("postgres".to_string());
    cfg.password = Some("codeharmony".to_string());
    cfg.dbname = Some("postgres".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let postgres_pool = cfg.create_pool(None, NoTls).unwrap();

    // Setup redis pool
    let cfg = deadpool_redis::Config::from_url("redis://127.0.0.1:6379");
    let redis_pool = cfg.create_pool(None).unwrap();

    // Setup lesson session server
    let server = ws_server::SessionServer::new().start();

    //Create and start server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(server.clone()))
            .service(coding_lesson::get_coding_lesson)
            .service(getusers)
            .route("/ws", web::get().to(ws_server::session_service))
            .configure(lesson_plan::init)
            .configure(lesson_session::init)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// #[derive(Serialize, Deserialize)]
// struct InfoObj {
//     elType:String
// }

// #[get("/infotest")]
// async fn infotest() -> impl Responder {
//     HttpResponse::Ok().json(InfoObj {
//         elType:String::from("h1")
//     })
// }

#[get("/users")]
async fn getusers(db_pool: web::Data<Pool>) -> impl Responder {
    let client = db_pool.get().await.unwrap();

    let statement = client
        .prepare("SELECT * FROM codeharmony.users")
        .await
        .unwrap();

    for row in client.query(&statement, &[]).await.iter() {
        println!("{:?}", row.get(0));
    }
    HttpResponse::Ok()
}
