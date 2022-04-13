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

use url::Url;

use endpoints::{account_management, lesson_plan, lesson_session, student_teacher};

use deadpool_postgres::{ManagerConfig, RecyclingMethod};
use dotenv::dotenv;

use crate::endpoints::{code_execution, publish_plan};

mod actors;
mod endpoints;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Get host, port and postgres url from env
    let host = env::var("HOST").expect("HOST not set!");
    let port = env::var("PORT").expect("PORT not set!");

    println!("Hosting on {}:{}", &host, &port);

    let postgres_pool = create_postgres_pool();
    let redis_pool = create_redis_pool();

    // Setup lesson session server
    let ws_session_server = SessionServer::new().start();

    // Teacher code actor
    let teacher_code_actor = TeacherCodeManager::new().start();

    //Create and start server
    HttpServer::new(move || {
        App::new()
            .wrap(create_cookie_session())
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(ws_session_server.clone()))
            .app_data(web::Data::new(teacher_code_actor.clone()))
            .route("/ws", web::get().to(session_service))
            .configure(lesson_plan::init)
            .configure(lesson_session::init)
            .configure(account_management::init)
            .configure(student_teacher::init)
            .configure(code_execution::init)
            .configure(publish_plan::init)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

fn create_cookie_session() -> CookieSession {
    CookieSession::signed(&[0; 32]).secure(false)
}

fn create_postgres_pool() -> deadpool_postgres::Pool {
    // Load .ENV file
    dotenv().ok();

    let postgres_url = env::var("DATABASE_URL").expect("DATABASE_URL not set!");
    let parsed_url = Url::parse(&postgres_url).expect("DATABASE_URL invalid!");

    let username = parsed_url.username();
    let password = parsed_url
        .password()
        .expect("Password not set in DATABASE_URL");
    let host = parsed_url.host_str().expect("Host not set in DATABASE_URL");
    let port = parsed_url.port().expect("Port not set in DATABASE_URL");
    let dbname = parsed_url
        .path_segments()
        .expect("DBName not set in DATABASE_URL")
        .next()
        .expect("DBName invalid id DATABASE_URL");

    // Setup Postgres pool
    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some(host.to_owned());
    cfg.port = Some(port.to_owned());
    cfg.user = Some(username.to_owned());
    cfg.password = Some(password.to_owned());
    cfg.dbname = Some(dbname.to_owned());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();

    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(config);

    cfg.create_pool(None, tls)
        .expect("Couldn't start postgres_pool")
}

fn create_redis_pool() -> deadpool_redis::Pool {
    // Load .ENV file
    dotenv().ok();

    // Setup redis pool
    let cfg = deadpool_redis::Config::from_url("redis://127.0.0.1:6379");
    cfg.create_pool(None).expect("Couldn't create redis pool")
}
