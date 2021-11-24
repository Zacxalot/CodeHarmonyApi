use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use deadpool_postgres::{ManagerConfig, RecyclingMethod, Pool};
use tokio_postgres::NoTls;



mod coding_lesson;
mod lesson_plan;
mod error;
mod jsx_element;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some("postgres".to_string());
    cfg.user = Some("postgres".to_string());
    cfg.password = Some("codeharmony".to_string());
    cfg.dbname = Some("postgres".to_string());
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });
    let pool = cfg.create_pool(None,NoTls).unwrap();


    // for row in pool.get().await.unwrap().query("SELECT * FROM USERS",&[]).await.unwrap(){
    //     println!("{:?}",row);
    // }

    //Configure DB Connection
    // let mut cfg = Config::from_env().unwrap();
    // let pool = cfg.pg.create_pool(NoTls).unwrap();

    //Create and start server
    HttpServer::new(move || {
        App::new().app_data(web::Data::new(pool.clone()))
        .service(coding_lesson::get_coding_lesson)
        .service(lesson_plan::create_lesson_plan)
        .service(getusers)
        .service(lesson_plan::get_plan_list)
        .service(lesson_plan::get_plan_info)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}


// #[derive(Serialize, Deserialize)]
// struct InfoObj {
//     el_type:String
// }


// #[get("/infotest")]
// async fn infotest() -> impl Responder {
//     HttpResponse::Ok().json(InfoObj {
//         el_type:String::from("h1")
//     })
// }


#[get("/users")]
async fn getusers(db_pool: web::Data<Pool>) -> impl Responder {
    let client = db_pool.get().await.unwrap();

    let statement = client.prepare(&"SELECT * FROM codeharmony.users").await.unwrap();

    for row in client.query(&statement,&[]).await.iter(){
        println!("{:?}",row.get(0));
    }
    HttpResponse::Ok()
}