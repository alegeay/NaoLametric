# Étape 1 : Compilation
FROM --platform=$BUILDPLATFORM rust:1.92.0-slim-trixie AS builder

ARG TARGETARCH
ARG TARGETPLATFORM
ARG BUILDPLATFORM

ENV CROSS_COMPILER_RELEASE=20250929
# La valeur ci-dessous est hardcodée car seule la cross-compilation de linux/amd64 -> linux/arm64
# est supportée. On ne peut donc télécharger qu'un seul compilateur.
ENV CROSS_COMPILER_SHA256=28a1d26f14f8ddc3aed31f20705fe696777400eb5952d90470a7e6e2dd1175bb

SHELL ["/bin/bash", "-c"]

RUN echo "I am running on $BUILDPLATFORM, building for $TARGETPLATFORM"

RUN apt update && \
    apt install -y --no-install-recommends wget xz-utils musl-dev upx-ucl && \
    rm -rf /var/cache/apt/lists && \
    rm -rf /var/cache/apt/archives

RUN case "$TARGETPLATFORM" in \
        linux/amd64) echo "x86_64-unknown-linux-musl" > /tmp/rust_target ; \
                     echo "x86_64" > /tmp/arch ; \
                     echo "export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc" > /tmp/cc_env ; \
                     echo "export CC=x86_64-unknown-linux-musl-gcc" >> /tmp/cc_env ;; \
        linux/arm64) echo "aarch64-unknown-linux-musl" > /tmp/rust_target ; \
                     echo "aarch64" > /tmp/arch ; \
                     echo "export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-unknown-linux-musl-gcc" > /tmp/cc_env ; \
                     echo "export CC=aarch64-unknown-linux-musl-gcc" >> /tmp/cc_env ;; \
        *) echo "Unsupported target arch: $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    rustup target add $(cat /tmp/rust_target)

RUN if [ $BUILDPLATFORM == $TARGETPLATFORM ]; then \
        echo -n "" > /tmp/cc_env ; \
    else \
        if [ $BUILDPLATFORM != "linux/amd64" ]; then \
            echo "Cross-compilation is only supported from linux/amd64 to linux/arm64" ; \
            # cf https://github.com/cross-tools/musl-cross/issues/13#issuecomment-3437856448
            exit 1 ; \
        fi ; \
        # Download a musl-targeting cross-compiler
        wget -q https://github.com/cross-tools/musl-cross/releases/download/20250929/$(cat /tmp/arch)-unknown-linux-musl.tar.xz ; \
        echo "$CROSS_COMPILER_SHA256 $(cat /tmp/arch)-unknown-linux-musl.tar.xz" | sha256sum --check --status ; \
        if [ $? -ne 0 ]; then \
            echo "Invalid checksum!" ; \
            exit 1 ; \
        fi ; \
        mkdir -p /opt/x-tools ; \
        tar xf $(cat /tmp/arch)-unknown-linux-musl.tar.xz -C /opt/x-tools ; \
        echo "export PATH=/opt/x-tools/$(cat /tmp/arch)-unknown-linux-musl/bin:$PATH" >> /tmp/cc_env ; \
    fi

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
    && source /tmp/cc_env \
    && cargo build --release --locked --target $RUST_TARGET \
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
