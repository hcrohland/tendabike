FROM rust:alpine AS base
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`

RUN apk add  musl-dev

WORKDIR /app

ENV DEBIAN_FRONTEND=noninteractive
# install nighlty toolchain
RUN rustup set profile minimal
# install dependencies
RUN cargo install cargo-chef
FROM base AS planner
# do not copy frontend!

COPY Cargo.toml Cargo.lock ./
COPY backend backend/

RUN cargo chef prepare --recipe-path recipe.json

FROM base AS cacher
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json


FROM cacher AS build-engine


ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx/
COPY backend backend/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp /app/target/release/tendabike /app/tendabike

FROM node:slim AS build-frontend

WORKDIR /frontend

COPY frontend/package.json frontend/package-lock.json /frontend/
RUN --mount=type=cache,target=/root/.npm \
    npm update rollup

COPY frontend/ /frontend
RUN --mount=type=cache,target=/root/.npm \
    npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/dist"

COPY --from=build-engine /app/tendabike ./
COPY --from=build-frontend /frontend/dist dist

ENTRYPOINT [ "./tendabike" ]
