# Étape 1 : Compilation
FROM rust:1.92.0-alpine3.22 AS builder

ARG TARGETARCH

RUN apk add --no-cache musl-dev upx

WORKDIR /app

# Cache des dépendances
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Compilation du binaire
COPY src ./src
RUN touch src/main.rs \
    && cargo build --release \
    && strip target/release/naolametric \
    && upx --best --lzma target/release/naolametric \
    && cp target/release/naolametric /naolametric-bin

# Étape 2 : Image scratch
FROM scratch

# Copier les certificats SSL pour HTTPS
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

COPY --from=builder /naolametric-bin /naolametric

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/naolametric"]
