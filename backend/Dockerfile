# --- Build ---
FROM rust:1.97-slim-trixie AS build
WORKDIR /app

# Cache dependencies separately from application code.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
    && cargo build --release --locked && rm -rf src

COPY src src
COPY migrations migrations
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# --- Runtime ---
FROM debian:trixie-slim
RUN useradd --system --uid 10001 --no-create-home soko
COPY --from=build /app/target/release/soko-backend /usr/local/bin/soko-backend
USER soko
ENV SOKO_BIND_ADDR=0.0.0.0:4000 \
    RUST_LOG=soko_backend=info,tower_http=info
EXPOSE 4000
CMD ["soko-backend"]
