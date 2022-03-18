use std::env;

use actix::Actor;
use actix_session::CookieSession;
use actix_web::{
    web::{self},
    App, HttpServer,
};
use actors::{
    teacher_code_manager::TeacherCodeManager,
    ws_server::{session_service, SessionServer},
};

use endpoints::{account_management, lesson_plan, lesson_session, student_teacher};

use deadpool_postgres::{ManagerConfig, RecyclingMethod};
use dotenv::dotenv;
use tokio_postgres::NoTls;

mod actors;
mod endpoints;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .ENV file
    dotenv().ok();

    // Get host address from env
    let addr = env::var("CH_HOST").unwrap_or_else(|_| "127.0.0.1:8080".into());
    println!("Hosting on: {}", &addr);

    let postgres_password =
        env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| panic!("POSTGRES_PASSWORD is undefined"));

    let postgres_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".into());
    println!("Postgres host: {}", &postgres_host);

    // Setup Postgres pool
    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some(postgres_host);
    cfg.user = Some("postgres".to_string());
    cfg.password = Some(postgres_password);
    cfg.dbname = Some("postgres".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let postgres_pool = cfg.create_pool(None, NoTls).unwrap();

    // Setup redis pool
    let cfg = deadpool_redis::Config::from_url("redis://127.0.0.1:6379");
    let redis_pool = cfg.create_pool(None).unwrap();

    // Setup lesson session server
    let ws_session_server = SessionServer::new().start();

    // Teacher code actor
    let teacher_code_actor = TeacherCodeManager::new().start();

    //Create and start server
    HttpServer::new(move || {
        App::new()
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(ws_session_server.clone()))
            .app_data(web::Data::new(teacher_code_actor.clone()))
            .route("/ws", web::get().to(session_service))
            .configure(lesson_plan::init)
            .configure(lesson_session::init)
            .configure(account_management::init)
            .configure(student_teacher::init)
    })
    .bind(addr)?
    .run()
    .await
}
