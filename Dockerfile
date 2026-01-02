# Étape 1 : Compilation
FROM rust:1.92.0-alpine3.22 AS builder

ARG TARGETARCH

RUN apk add --no-cache musl-dev upx

# Déterminer la target en fonction de l'architecture
# Si TARGETARCH n'est pas défini (legacy builder), on détecte l'arch native
RUN if [ -z "$TARGETARCH" ]; then \
        case "$(uname -m)" in \
            x86_64) TARGETARCH=amd64 ;; \
            aarch64) TARGETARCH=arm64 ;; \
        esac; \
    fi && \
    case "$TARGETARCH" in \
        amd64) echo "x86_64-unknown-linux-musl" > /tmp/rust_target ;; \
        arm64) echo "aarch64-unknown-linux-musl" > /tmp/rust_target ;; \
        *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add $(cat /tmp/rust_target)

WORKDIR /app

# Cache des dépendances
COPY Cargo.* ./
RUN RUST_TARGET=$(cat /tmp/rust_target) \
    && mkdir src && echo "fn main() {}" > src/main.rs \
    && source /tmp/cc_env \
    && cargo build --release --locked --target $RUST_TARGET \
    && rm -rf src

# Compilation du binaire
COPY src ./src
RUN RUST_TARGET=$(cat /tmp/rust_target) \
    && touch src/main.rs \
    && cargo build --release --locked --target $RUST_TARGET \
    && strip target/$RUST_TARGET/release/naolametric \
    && upx --best --lzma target/$RUST_TARGET/release/naolametric \
    && cp target/$RUST_TARGET/release/naolametric /naolametric-bin

# Étape 2 : Image scratch
FROM scratch

# Copier les certificats SSL pour HTTPS
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

COPY --from=builder /naolametric-bin /naolametric

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/naolametric"]
