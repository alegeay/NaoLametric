# Étape 1 : Compilation avec nightly + build-std
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev upx

# Installer nightly et rust-src
RUN rustup toolchain install nightly \
    && rustup component add rust-src --toolchain nightly

WORKDIR /app

# Cache des dépendances
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && RUSTFLAGS="-Zunstable-options -Cpanic=abort" cargo +nightly build --release \
       -Z build-std=std,panic_abort \
       --target x86_64-unknown-linux-musl \
    && rm -rf src

# Compilation du binaire avec build-std
COPY src ./src
RUN touch src/main.rs \
    && RUSTFLAGS="-Zunstable-options -Cpanic=abort" cargo +nightly build --release \
       -Z build-std=std,panic_abort \
       --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/naolametric \
    && upx --best --lzma target/x86_64-unknown-linux-musl/release/naolametric

# Étape 2 : Image scratch
FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/naolametric /naolametric

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/naolametric"]
