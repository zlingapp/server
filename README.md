# 🦀 Zling API Server
This is the monolithic API server which hosts Zling's functions. It's written in Rust with the help of `actix-web`, `mediasoup-rust` and `sqlx`.

> Zling is currently in development. Expect the API to change. API Docs are virtually non existent at the moment.

### Architecture
- Database: PostgreSQL
- Voice SFU: Mediasoup
- Websockets: `actix-ws`

### Features
- User management
- Text messaging
- Voice chat relay
- File Upload & Download (TODO)
- HMAC-SHA256 based token authentication: no query overhead for validating access tokens
- Real-time pub/sub event system
- Stateless (except for voice); all data stored in database

### Object IDs
Zling uses the `nanoid` crate to generate URL-safe NanoIDs, of the default length of 21.

### Tokens
Zling uses an access and refresh token architecture.

#### Samples
|Type|Example|Validity|
|-|-|-|
|Access Token|`TksgHm2VlVGauu-idaO4w.ZGdKYQ.OHMHwz6l3XkHSYOSns8IHtxxi_sHBrzmYu0gqWZtcUs`| 10 mins
|Refresh Token|`TksgHm2VlVGauu-idaO4w.ZGs8iQ.1ZcETwSXSEqeB6O19C0J_GOgFg8UeHrVv56QmGsszHmUDSog`|3 days

You can issue yourself a token by signing in at the `/auth/login` endpoint with a username and password, and then call `/auth/reissue` with a refresh token to obtain new access & refresh tokens accordingly.

#### Token Structure
The format of tokens resembles JWT but is not compliant for the sake of shortness. Tokens are of format 
 ```
    xoKM4W7NDqHjK_V0g9s3y.ZFZDYw.iIuDsgiT4s2ehQ-3ATImimyPUoooTPC1ytqqQuPQSJU

    AAAAAAAAAAAAAAAAAAAAA.BBBBBB.CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
    ~~~~~~~~~~~~~~~~~~~~~ ~~~~~~ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
            user_id       expiry                 signature
 ```

 Where 
 ```
 expiry = BASE64URL(unix_timestamp.big_endian_bytes)

 signature = BASE64URL(
    HMACSHA256_SIGN(
        user_id + "." + BASE64URL(expiry), 
        TOKEN_SIGNING_KEY
    )
 )
 ```
 Note: `user_id` is **not** Base64 encoded as it is already url-safe.
 
 Note: `expiry` is a Unix timestamp in seconds, encoded as a Base64Url string. Bytes are encoded in Big Endian (network order).

 In the example:
 - `user_id` = `xoKM4W7NDqHjK_V0g9s3y`
 - `expiry` = `BASE64URL_DECODE("ZFZDYw") = 0x64564363 = 1683374947 (big-endian) = Sat May 06 2023 12:09:07 GMT+0000`

### Environment Variables
See [the options.rs file](src/options.rs).

#### Database
|Variable|Type|Default|
|-|-|-|
|`DB_HOST`|`String`|`localhost`|
|`DB_PORT`|`u16`|`5432`|
|`DB_USER`|`String`|`zling-backend`|
|`DB_PASSWORD`|`String`|`dev`|
|`DB_NAME`|`String`|`zling-backend`|
|`DB_POOL_MAX_CONNS`|`u32`|`5`|

#### Voice Chat
|Variable|Type|Default|
|-|-|-|
|`RTC_PORT_MIN`|`u16`|`10000`|
|`RTC_PORT_MAX`|`u16`|`11000`    |
|`ANNOUNCE_IP`|`IpAddr`|`127.0.0.1`|
|`ENABLE_UDP`|`bool`|`true`|
|`ENABLE_TCP`|`bool`|`true`|
|`PREFER_UDP`|`bool`|`true`|
|`PREFER_TCP`|`bool`|`false`|
|`INITIAL_AVAILABLE_OUTGOING_BITRATE`|`u32`|`600000`|

#### Token Generation
|Variable|Type|Default|
|-|-|-|
|`TOKEN_SIGNING_KEY`|`String`|Randomly generated|
|`PRINT_GENERATED_TOKEN_SIGNING_KEY`|`bool`|`false`|

#### Miscellaneous
- Recommended: start the server with `RUST_LOG=info,sqlx::query=warn`.