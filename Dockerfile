FROM rust:1.99.1-trixie AS builder
COPY --from=oven/bun:1.4.2 /usr/local/bin/bun /usr/local/bin/bun
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY frontend/package.json frontend/bun.lock frontend/
RUN cd frontend && bun install --frozen-lockfile

COPY . .
RUN touch src/main.rs && cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/chipper /usr/local/bin/chipper
USER nobody
EXPOSE 8080
CMD ["chipper"]
