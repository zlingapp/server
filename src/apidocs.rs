use utoipa::{OpenApi, openapi::security::{SecurityScheme, HttpBuilder, HttpAuthScheme, ApiKey, ApiKeyValue}};

use crate::{
    auth::routes::AuthApiDocs, channels::routes::ChannelsApiDocs, guilds::routes::GuildsApiDocs, media::routes::MediaApiDocs, messaging::routes::MessagingApiDocs, voice::routes::VoiceApiDoc, realtime::pubsub::PubSubApiDoc,
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
    oapi.merge(MediaApiDocs::openapi());
    oapi.merge(MessagingApiDocs::openapi());
    oapi.merge(VoiceApiDoc::openapi());
    oapi.merge(PubSubApiDoc::openapi());

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

struct VoiceSecurityAddon;

impl utoipa::Modify for VoiceSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.components = Some(
            utoipa::openapi::ComponentsBuilder::new()
                .security_scheme(
                    "voice",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("RTC-Token")))
                )
                .build(),
        )
    }
}
