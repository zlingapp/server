use actix_web::{test, web::Data, App};
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
        .set_json(json!({
            "email": "test@example.com",
            "password": "password123",
            "username": "Test User",
        }))
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

#[sqlx::test(fixtures("users"))]
async fn test_login_whoami(pool: Pool<Postgres>) {
    let db = Data::new(Database::with_pool(pool.clone()));
    let app = test::init_service(
        App::new()
            .app_data(db)
            .configure(auth::routes::configure_app),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/auth/login")
        .set_json(json!({
            "email": "jthomas@example.com",
            "password": "password123",
        }))
        .to_request();
    let resp: auth::routes::login::LoginResponse = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.user.name, "James#1119");

    let req = test::TestRequest::get()
        .uri("/auth/whoami")
        .insert_header((
            "Authorization",
            "Bearer ".to_owned() + &resp.access_token.to_string(),
        ))
        .to_request();
    let resp: auth::user::User = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.name, "James#1119");
    assert_eq!(resp.email, Some("jthomas@example.com".into()));
    assert_eq!(resp.bot, false);
    assert_eq!(resp.id, "oUR5y3ph_EFASnt037BCK");
}
