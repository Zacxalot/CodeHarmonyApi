use std::{env, fs};

use actix::Actor;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    web::{self},
    App, HttpServer,
};
use actors::{
    teacher_code_manager::TeacherCodeManager,
    ws_server::{session_service, SessionServer},
};

use native_tls::{Certificate, TlsConnector};
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
    // Load .ENV file
    dotenv().ok();

    // Get host, port and postgres url from env
    let host = env::var("HOST").expect("HOST not set!");
    let port = env::var("PORT").expect("PORT not set!");

    println!("Hosting on {}:{}", &host, &port);

    let postgres_pool = create_postgres_pool().await;

    // Setup lesson session server
    let ws_session_server = SessionServer::new().start();

    // Teacher code actor
    let teacher_code_actor = TeacherCodeManager::new().start();

    //Create and start server
    HttpServer::new(move || {
        App::new()
            .wrap(create_cookie_session())
            .app_data(web::Data::new(postgres_pool.clone()))
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

fn create_cookie_session() -> SessionMiddleware<CookieSessionStore> {
    // Load .ENV file
    dotenv().ok();

    // Get session key
    let session_key = env::var("SESSION_KEY").expect("SESSION_KEY not set!");
    let session_key = Key::from(session_key.as_bytes());

    SessionMiddleware::new(CookieSessionStore::default(), session_key)
}

async fn create_postgres_pool() -> deadpool_postgres::Pool {
    // Load .ENV file
    dotenv().ok();

    println!("Loaded env");

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

    println!("Got vars");

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
    cfg.ssl_mode = Some(deadpool_postgres::SslMode::Require);
    cfg.application_name = Some("Code_Harmony server".to_owned());

    println!("Created config");

    let cert = fs::read("CA_CERT.pem").expect("Couldn't find CA_CERT.pem");

    println!("Opened CA_CERT.pem");

    let cert = Certificate::from_pem(&cert).expect("Couldn't parse CA cert");

    println!("Read certs");

    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .danger_accept_invalid_hostnames(true)
        .build()
        .expect("Couldn't build connector");
    let connector = postgres_native_tls::MakeTlsConnector::new(connector);

    println!("Created connector");

    let pool = cfg
        .create_pool(None, connector)
        .expect("Couldn't start postgres_pool");

    println!("Postgres pool created");

    match pool.get().await {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
            panic!("Couldn't connect to database")
        }
    }

    println!("Postgres pool valid");

    pool
}
