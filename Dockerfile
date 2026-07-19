FROM rust:1.88-bookworm-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/customer-transactions /usr/local/bin/customer-transactions
EXPOSE 8080
CMD ["customer-transactions"]
