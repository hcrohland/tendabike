# Shared rust base: C linker deps + slim rustup profile. Both cargo
# stages inherit from it so these layers are built once and cached
# identically.
FROM rust:alpine AS rust-base
RUN apk add  musl-dev
RUN rustup set profile minimal

FROM rust-base AS chef
RUN apk add cargo-chef   # or: cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-deps
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json

# Build stage: starts from the pre-compiled target directory, so cargo
# recompiles only the seven real workspace crates.
FROM rust-base AS build-engine

WORKDIR /app
COPY --from=build-deps /app/target ./target
COPY --from=build-deps /usr/local/cargo/registry /usr/local/cargo/registry

ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY backend/sqlx/.sqlx backend/sqlx/.sqlx/
COPY backend backend/
# The copied target directory carries cargo's fingerprints from the stub
# build: each unit's invoked.timestamp is the stub build time, which is
# always newer than the build-context mtimes that COPY preserves (the git
# checkout). Cargo's mtime check would then call every workspace source
# unchanged and ship the stub binary as-is. Touching the sources past
# those fingerprints forces the five workspace crates to recompile; the
# registry dependencies are untouched and stay cached.
RUN find backend -type f -name '*.rs' -exec touch {} + \
    && cargo build --release \
    && cp /app/target/release/tendabike /app/tendabike

FROM node:slim AS build-frontend

WORKDIR /build

COPY package.json package-lock.json /build/
COPY frontend/package.json /build/frontend/
RUN npm update rollup

COPY frontend/ /build/frontend
RUN npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/build"

COPY --from=build-engine /app/tendabike ./
COPY --from=build-engine /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build-frontend /build/frontend/build build

ENTRYPOINT [ "./tendabike" ]
