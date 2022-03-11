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
use tokio_postgres::NoTls;

mod actors;
mod endpoints;
mod utils;

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
    let server = SessionServer::new().start();

    // Teacher code actor
    let teacher_code_actor = TeacherCodeManager::new().start();

    //Create and start server
    HttpServer::new(move || {
        App::new()
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(server.clone()))
            .app_data(web::Data::new(teacher_code_actor.clone()))
            .route("/ws", web::get().to(session_service))
            .configure(lesson_plan::init)
            .configure(lesson_session::init)
            .configure(account_management::init)
            .configure(student_teacher::init)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
