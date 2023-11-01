use actix_web::{dev::ServiceResponse, test, web::Data, App};
use serde::de::DeserializeOwned;
use serde_json::json;
use sqlx::{query, Pool, Postgres};
use std::env::set_var;
use zling_server::{
    auth,
    auth::{routes::login::LoginResponse, user::User},
    db::Database,
};

#[sqlx::test]
async fn test_registration(pool: Pool<Postgres>) {
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
    assert!(
        resp.status().is_success(),
        "registration status code {}",
        resp.status().as_str()
    );

    let user = query!(r#"SELECT * FROM users"#)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        user.email,
        Some("test@example.com".into()),
        "testing whether the database contains the correct user"
    );
    assert_eq!(user.name.split("#").next(), Some("Test User"));
}

#[sqlx::test(fixtures("users"))]
async fn test_login(pool: Pool<Postgres>) {
    let db = Data::new(Database::with_pool(pool));
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
    let resp = test::call_service(&app, req).await;
    let resp: LoginResponse = resp_deserialize(resp).await;
    assert_eq!(
        resp.user.name, "James#1119",
        "testing response from initial login"
    );

    let req = test::TestRequest::get()
        .uri("/auth/whoami")
        .insert_header((
            "Authorization",
            "Bearer ".to_owned() + &resp.access_token.to_string(),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let resp: User = resp_deserialize(resp).await;
    assert_eq!(
        resp.name, "James#1119",
        "testing that whoami returns the correct user"
    );
    assert_eq!(resp.email, Some("jthomas@example.com".into()));
    assert_eq!(resp.bot, false);
    assert_eq!(resp.id, "oUR5y3ph_EFASnt037BCK");
}

#[sqlx::test(fixtures("users", "tokens"))]
async fn test_whoami(pool: Pool<Postgres>) {
    // TODO find some way to set this globally in integration tests
    // if cfg!(test) only works for unit tests, not calls from lib
    set_var(
        "TOKEN_SIGNING_KEY",
        "3c6099c189b60fc2843d7059d33b1de1bc05d3b5b6d74bf249db4d8dcd844e62",
    );
    let db = Data::new(Database::with_pool(pool));
    let app = test::init_service(
        App::new()
            .app_data(db.clone())
            .configure(auth::routes::configure_app),
    )
    .await;
    let req = privileged_get("/auth/whoami").to_request();
    let resp = test::call_service(&app, req).await;
    let resp: User = resp_deserialize(resp).await;
    assert_eq!(resp.name, "James#1119");
    assert_eq!(resp.email, Some("jthomas@example.com".into()));
    assert_eq!(resp.bot, false);
    assert_eq!(resp.id, "oUR5y3ph_EFASnt037BCK");
}

/// actix_web will just throw a deserialization error if you dont get a 200 code.
/// This makes it a bit more readable.
async fn resp_deserialize<T>(resp: ServiceResponse) -> T
where
    T: DeserializeOwned,
{
    assert!(
        resp.status().is_success(),
        "Call to {} failed with status code {}",
        resp.request().uri(),
        resp.status().as_str()
    );
    test::read_body_json(resp).await
}

// Attach necessary headers to make a privileged call (using fixed token and user from /fixtures)
fn privileged_get(endpoint: &str) -> actix_web::test::TestRequest {
    test::TestRequest::get()
        .uri(endpoint)
        .insert_header((
            "Authorization",
            "Bearer oUR5y3ph_EFASnt037BCK.ZUUnGQ.J16TLUl6Zg07tj413e2zJU8Qe3A7P6Aiq50qwl-t42o",
        ))
        .insert_header(("User-Agent", "test"))
}
