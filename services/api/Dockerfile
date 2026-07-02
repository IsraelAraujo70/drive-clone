FROM rust:1.95-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/drive-clone-api /usr/local/bin/drive-clone-api

ENV HOST=0.0.0.0
ENV PORT=8080

CMD ["/usr/local/bin/drive-clone-api"]
