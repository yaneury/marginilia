FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/marginilia-server ./marginilia-server
COPY quotes.yaml ./quotes.yaml

ENV QUOTES_PATH=/app/quotes.yaml
ENV PORT=3000
EXPOSE 3000

CMD ["./marginilia-server"]
