# 🦀 Zling API Server
This is the monolithic API server which hosts Zling's functions. It's written in Rust with the help of `actix-web`, `mediasoup-rust` and `sqlx`.

> Zling is currently in development. Expect the API to change. API Docs are hosted by the server at /docs.

### Architecture
- Database: PostgreSQL (`sqlx`)
- Voice SFU: `mediasoup`
- HTTP: `actix-web`
- Websockets: `actix-ws`

### Features
- Text messaging
- Voice chat
- File attachments
- Performant
- Fully documented with OpenAPI

## Reference

#### Object IDs
Zling uses the `nanoid` crate to generate URL-safe NanoIDs, of the default length of 21.

#### Tokens
Zling uses an access and refresh token architecture for authorization. It's similar in ways 
to the tokens used by OAuth2, but is not strictly compliant. 

##### What the server knows
The database contains a table of all refresh tokens and their expiry, which is used to renew a token pair.
The server does **not** keep track of all valid access tokens, as it determines an access token's 
authenticity with HMAC verification.

##### Generating a token signing key
By default, Zling's server generates a random token signing key between restarts, which deauthenticates any existing
access tokens your clients might be using. If you want access tokens to be valid between runs, you need to generate a 
token signing key to be used persistently. 

Start the server with `TOKEN_SIGNING_KEY` unset to get a random token. You may
start the server with `cargo run`, `cargo run --release`, or just use the
pre-built binary.

```
$ cargo run

[...] Version: 0.1.0
[...] Generating new token signing key... (provide one with TOKEN_SIGNING_KEY)
[...] Token signing key: d8b9e886234d4500dont_use_this_readmes_key_in_productiona02f9f3bff03
*logs continue below*
```

Now, restart the server with `TOKEN_SIGNING_KEY` set to the generated token:
```
$ TOKEN_SIGNING_KEY=xxxxx cargo run --release
```
Now you can try logging in on a client, restarting the server, then accessing the server again from the client. If you did 
everything correctly, you won't have to log in again.

##### What is a token made of?
|Type|Example|Validity|
|-|-|-|
|Access Token|`TksgHm2VlVGauu-idaO4w.ZGdKYQ.OHMHwz6l3XkHSYOSns8IHtxxi_sHBrzmYu0gqWZtcUs`| Short (~10 mins)
|Refresh Token|`TksgHm2VlVGauu-idaO4w.ZGs8iQ.1ZcETwSXSEqeB6O19C0J_GOgFg8UeHrVv56QmGsszHmUDSog`| Long (~3 days)

You can issue yourself a token by signing in at the `/auth/login` endpoint with a username and password, and then call `/auth/reissue` with a refresh token to obtain new access & refresh tokens accordingly.

The format of tokens resembles JWT but is not compliant for the sake of
shortness. Tokens are of the format:
```
   xoKM4W7NDqHjK_V0g9s3y.ZFZDYw.iIuDsgiT4s2ehQ-3ATImimyPUoooTPC1ytqqQuPQSJU

   AAAAAAAAAAAAAAAAAAAAA.BBBBBB.CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
   ~~~~~~~~~~~~~~~~~~~~~ ~~~~~~ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
         user_id       expiry                 signature
```

Where 
```
expiry = BASE64URL(unix_timestamp.big_endian_bytes)
payload = user_id + "." + expiry
signature = BASE64URL(HMACSHA256_SIGN(payload, TOKEN_SIGNING_KEY))

token = payload + "." + signature
```
Note: `user_id` is **not** Base64 encoded as it is already url-safe.

Note: `expiry` is a Unix timestamp in seconds, encoded as a Base64Url string. Bytes are encoded in Big Endian (network order).

In the example:
 - `user_id` = `xoKM4W7NDqHjK_V0g9s3y`
 - `expiry` = `BASE64URL_DECODE("ZFZDYw") = 0x64564363 = 1683374947 (big-endian) = Sat May 06 2023 12:09:07 GMT+0000`

### Configuration
The server is configured through the following environment variables.
See [the options.rs file](src/options.rs) for details.

#### API
|Variable|Type|Default|Description|
|-|-|-|-|
|`NUM_WEB_WORKERS`|`number`|`4`|Web worker threads that should be spawned. Set to number of availabe threads available for best performance. eg. `NUM_WEB_WORKERS=$(nproc)`|
|`BIND_ADDR`|`ipv4:port`|`127.0.0.1:8080`|HTTP bind address. Set to `0.0.0.0:1234` to listen to any address on port `1234`.|
|`SSL_ENABLE`|`bool`|`false`|Should an HTTPS server be started? Required: `SSL_CERT_PATH`, `SSL_KEY_PATH`|
|`SSL_ONLY`|`bool`|`false`|Should the HTTP server be disabled in favour of just the HTTPS server alone? Required: `SSL_ENABLE`|
|`SSL_BIND_ADDR`|`ipv4:port`|`127.0.0.1:8443`|HTTPS bind address. Set to `0.0.0.0:8443` to listen to any address on port `8443`.|
|`SSL_CERT_PATH`|`path`|`cert.pem`|Your TLS certificate file, required for HTTPS support. You can get one from Let's Encrypt using certbot for free.|
|`SSL_KEY_PATH`|`path`|`key.pem`|Your TLS private key file, required for HTTPS support. You can get one from Let's Encrypt using certbot for free.|
|`HANDLE_CORS`|`bool`|`true`|Attach necessary Cross Origin Access Control (CORS) headers to allow any website domain to make requests to this instance. Useful for when you want to use https://zlingapp.com as the app and add your instance to it. You can also do this with a reverse proxy like NGINX.|

#### Database
|Variable|Type|Default|Description|
|-|-|-|-|
|`DB_HOST`|`string`|`localhost`|PostgreSQL hostname|
|`DB_PORT`|`number`|`5432`|PostgreSQL port|
|`DB_USER`|`string`|`zling-backend`|PostgreSQL user|
|`DB_PASSWORD`|`string`|`dev`|Password for user|
|`DB_NAME`|`string`|`zling-backend`|Database name|
|`DB_POOL_MAX_CONNS`|`number`|`5`|Max open query connections|

#### Voice Chat
|Variable|Type|Default|Description|
|-|-|-|-|
|`WRTC_PORTS`|`range`|`10000`|Port range to start WebRTC servers on. Don't specify too many as each port starts its own WebRTC server! A recommended amount is `2-4`. (eg. `WRTC_PORTS=10000-10003`)|
|`WRTC_ANNOUNCE_IP`|`ipv4`|`127.0.0.1`|Public IP with which WebRTC clients should seek a connection. It's important that this IP is your server's public IP and `WRTC_PORTS` are accessible on it, otherwise voice chat won't work.|
|`WRTC_ENABLE_UDP`|`bool`|`true`|Clients can use UDP to send voice packets? (recommended)|
|`WRTC_ENABLE_TCP`|`bool`|`true`|Clients can use UDP to send voice packets? (recommended for fallback as some networks disallow UDP altogether)|
|`WRTC_PREFER_UDP`|`bool`|`true`|Should UDP be preferred over TCP? (recommended)|
|`WRTC_PREFER_TCP`|`bool`|`false`|Should TCP be preferred over UDP?|
|`WRTC_INITIAL_AVAILABLE_OUTGOING_BITRATE`|`number`|`600000`|Initial value for the outbound bitrate limit when negotiating WebRTC link speed. 600kbps as the default should suit most cases.|

#### Access Token Signing
|Variable|Type|Default|Description|
|-|-|-|-|
|`TOKEN_SIGNING_KEY`|`hex string`|Randomly generated|Key used to sign access tokens, see above for format.|

#### User media files
|Variable|Type|Default|Description|
|-|-|-|-|
|`MEDIA_PATH`|`path`|`/var/tmp/zling-media`|Directory where user files like avatars and attachments should be stored. Ideally it should have a lot of capacity.|

### Database migrations
Database migrations are handled simply with `sqlx-cli` and the `/migrations` directory. On the first `sqlx migrate run`, each `.up` file is run in succession according to their timestamp. On any subsequent run, only new migrations are run, allowing an existing database to be modified non-destructively. Additionally, any change can be reverted using `sqlx revert`, running the `.down` sql file. Any `sortableInt_name.up.sql` file can be used, but ideally create migrations using `sqlx migrate add`.
