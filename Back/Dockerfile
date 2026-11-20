# syntax=docker/dockerfile:1
# API image (ADR-0012): multi-stage build, distroless non-root runtime.

FROM rust:1-bookworm AS build
WORKDIR /src
# sqlx query macros use the committed .sqlx metadata, no database needed at build time.
ENV SQLX_OFFLINE=true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
    && cp target/release/gite-api /gite-api

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /gite-api /usr/local/bin/gite-api
# Technical defaults for a container behind the Caddy proxy; secrets come from the environment.
ENV APP_ENV=production \
    BIND_ADDR=0.0.0.0:8080 \
    LOG_FORMAT=json \
    TRUST_PROXY_HEADERS=true
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/gite-api"]
CMD ["serve"]
