use actix_web::{http::header::ContentType, test, web::Data, App};
use serde_json::json;
use sqlx::{query, Pool, Postgres};
use zling_server::{auth, db::Database};

#[sqlx::test]
async fn test_successful_registration(pool: Pool<Postgres>) {
    let db = Data::new(Database::with_pool(pool.clone()));
    let app = test::init_service(
        App::new()
            .app_data(db)
            .configure(auth::routes::configure_app),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(json!( {
            "email": "test@example.com",
            "password": "password123",
            "username": "Test User",
        }))
        .insert_header(ContentType::json())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let user = query!(r#"SELECT * FROM users"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    
    assert_eq!(user.email, Some("test@example.com".into()));
    assert_eq!(user.name.split("#").collect::<Vec<&str>>()[0], "Test User");
}
