FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src
COPY . .
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/cartobase /usr/local/bin/cartobase
COPY --from=build /app/web ./web
COPY --from=build /app/migrations ./migrations
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["cartobase"]
