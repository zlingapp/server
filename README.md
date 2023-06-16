# 🦀 Zling API Server
This is the monolithic API server which hosts Zling's functions. It's written in Rust with the help of `actix-web`, `mediasoup-rust` and `sqlx`.

> Zling is currently in development. Expect the API to change. API Docs are virtually non existent at the moment.

### Architecture
- Database: PostgreSQL (`sqlx`)
- Voice SFU: `mediasoup`
- HTTP: `actix-web`
- Websockets: `actix-ws`
- Written in Rust (holy moly)

### Features
- User management
- Text messaging
- Voice chat
- File Upload & Download
- Real-time pub/sub event system
- Stateless authentication
- Really, really fast, thanks to Rust and Actix-web

## Reference

### Object IDs
Zling uses the `nanoid` crate to generate URL-safe NanoIDs, of the default length of 21.

### Tokens
Zling uses an access and refresh token architecture for authorization. It's similar in ways 
to the tokens used by OAuth2, but is not strictly compliant. 

#### What the server knows
The database contains a table of all refresh tokens and their expiry, which is used to renew a token pair.
The server does **not** keep track of all valid access tokens, as it determines an access token's 
authenticity with cryptographic signature verification.

#### Generating a token signing key
By default, Zling's server generates a random token signing key between restarts, which deauthenticates any existing
access tokens your clients might be using. If you want access tokens to be valid between runs, you need to generate a 
token signing key to be used persistently. 

Start the server with `TOKEN_SIGNING_KEY` unset to get a random token. You may
start the server with `cargo run`, `cargo run --release`, or just use the
pre-built binary.

Make sure `RUST_LOG` is at least set to `info`, so you can actually see the token in the log output. 
```
$ RUST_LOG=info cargo run

[...] Version: 0.1.0
[...] Generating new token signing key... (provide one with TOKEN_SIGNING_KEY)
[...] Token signing key: d8b9e886234d4500dont_use_this_readmes_key_in_productiona02f9f3bff03
*logs continue below*
```

Now, restart the server with `TOKEN_SIGNING_KEY` set to the generated token:
```
$ RUST_LOG=info,sqlx::query=warn TOKEN_SIGNING_KEY=xxxxx cargo run --release
```
Now you can try logging in on a client, restarting the server, then accessing the server again from the client. If you did 
everything correctly, you won't have to log in again.

#### What is a token made of?
|Type|Example|Validity|
|-|-|-|
|Access Token|`TksgHm2VlVGauu-idaO4w.ZGdKYQ.OHMHwz6l3XkHSYOSns8IHtxxi_sHBrzmYu0gqWZtcUs`| Short (~10 mins)
|Refresh Token|`TksgHm2VlVGauu-idaO4w.ZGs8iQ.1ZcETwSXSEqeB6O19C0J_GOgFg8UeHrVv56QmGsszHmUDSog`| Long (~3 days)

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

#### Access Token Signing
|Variable|Type|Default|
|-|-|-|
|`TOKEN_SIGNING_KEY`|`String`|Randomly generated|

#### User media files
|Variable|Type|Default|
|-|-|-|
|`MEDIA_PATH`|`String`|`/var/tmp/zling-media`|

#### Miscellaneous
- Recommended: start the server with `RUST_LOG=info,sqlx::query=warn`.
