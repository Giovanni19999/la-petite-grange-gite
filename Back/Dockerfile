# syntax=docker/dockerfile:1
# API image (ADR-0012, ADR-0016): static musl binary on a distroless "static" runtime, so the
# image ships no C library, OpenSSL or C++ runtime at all (TLS is rustls, see Cargo.toml).

FROM rust:1-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add "$(uname -m)-unknown-linux-musl"
WORKDIR /src
# sqlx query macros use the committed .sqlx metadata, no database needed at build time.
ENV SQLX_OFFLINE=true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    target="$(uname -m)-unknown-linux-musl" \
    && cargo build --release --locked --target "$target" \
    && cp "target/$target/release/gite-api" /gite-api \
    && mkdir -p /media/public /media/originals

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /gite-api /usr/local/bin/gite-api
# Media directory owned by the non-root user, so that a new volume mounted here inherits it.
COPY --from=build --chown=65532:65532 /media /data/media
# Technical defaults for a container behind the Caddy proxy; secrets come from the environment.
ENV APP_ENV=production \
    BIND_ADDR=0.0.0.0:8080 \
    LOG_FORMAT=json \
    TRUST_PROXY_HEADERS=true \
    MEDIA_DIR=/data/media
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/gite-api"]
CMD ["serve"]
