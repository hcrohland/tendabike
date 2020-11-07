FROM rust:latest as base
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`

WORKDIR /app
# install dependencies
RUN apt-get update && apt-get install -y libpq-dev libssl-dev

ENV DEBIAN_FRONTEND=noninteractive
RUN cargo install cargo-chef 
# install nighlty toolchain
RUN rustup update nightly && rustup default nightly

FROM base as planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM base as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


FROM cacher as build-engine

# do not copy tb_svelte!
COPY Cargo.toml Cargo.lock ./
COPY tb_engine/ ./tb_engine/
COPY tb_common/ ./tb_common/
COPY tb_strava/ ./tb_strava/

RUN cargo build --release

FROM node:15-alpine AS build-frontend

WORKDIR /tb_svelte

COPY tb_svelte/package.json tb_svelte/package-lock.json /tb_svelte/
RUN npm install

COPY tb_svelte/ /tb_svelte
RUN npm run build

FROM debian:buster-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates

RUN useradd --system tendabike
USER tendabike
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/public"

COPY --from=build-engine /app/target/release/tb_engine ./
COPY --from=build-frontend /tb_svelte/public/* ./public/

ENTRYPOINT [ "./tb_engine" ]
