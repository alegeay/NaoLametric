# Build stage
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual binary
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates libssl3

WORKDIR /app

COPY --from=builder /app/target/release/naolametric /app/naolametric

ENV PORT=8080

EXPOSE 8080

CMD ["/app/naolametric"]
