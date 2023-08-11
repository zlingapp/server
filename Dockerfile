# cargo chef
FROM lukemathwalker/cargo-chef:latest-rust-1 as chef
WORKDIR /app

FROM chef as planner
COPY . .
# Create recipe.json file
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
RUN apt-get update && apt-get install -y python3-pip
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
# WARNING: this will fail without `--network=host` on docker build
# because sqlx wants to talk to the database at 127.0.0.1:5432 to generate code
# database should be started with ./dev/setup.sh
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app

USER 1000
COPY --from=builder /app/target/release/zling-server /app/server

ENTRYPOINT ["/app/server"]