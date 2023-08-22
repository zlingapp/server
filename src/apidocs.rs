use utoipa::{OpenApi, openapi::security::{SecurityScheme, HttpBuilder, HttpAuthScheme}};

use crate::{
    auth::routes::AuthApiDocs, channels::routes::ChannelsApiDocs, guilds::routes::GuildsApiDocs,
};

#[derive(OpenApi)]
#[openapi(
    modifiers(&TokenSecurityAddon)
)]
pub struct ApiDocs;

pub fn setup_oapi() -> utoipa::openapi::OpenApi {
    let mut oapi = ApiDocs::openapi();

    oapi.merge(AuthApiDocs::openapi());
    oapi.merge(ChannelsApiDocs::openapi());
    oapi.merge(GuildsApiDocs::openapi());

    oapi
}

struct TokenSecurityAddon;

impl utoipa::Modify for TokenSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.components = Some(
            utoipa::openapi::ComponentsBuilder::new()
                .security_scheme(
                    "token",
                    SecurityScheme::Http(
                        HttpBuilder::new()
                            .scheme(HttpAuthScheme::Bearer)
                            .bearer_format("AccessToken")
                            .build(),
                    ),
                )
                .build(),
        )
    }
}
