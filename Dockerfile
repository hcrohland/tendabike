FROM lukemathwalker/cargo-chef:latest as base
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`

WORKDIR /app

ENV DEBIAN_FRONTEND=noninteractive
# install nighlty toolchain
RUN rustup set profile minimal
# install dependencies
RUN apt-get update && apt-get install -y libpq-dev

FROM base as planner
# do not copy frontend!
COPY Cargo.toml Cargo.lock ./
COPY app app/
COPY axum axum/
COPY diesel diesel/
COPY domain domain/
COPY strava strava/


RUN cargo chef prepare --recipe-path recipe.json


FROM base as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


FROM cacher as build-engine

# do not copy frontend!
COPY Cargo.toml Cargo.lock ./
COPY app app/
COPY axum axum/
COPY diesel diesel/
COPY domain domain/
COPY strava strava/

RUN cargo build --release

FROM node:21 AS build-frontend

WORKDIR /frontend

COPY frontend/package.json frontend/package-lock.json /frontend/
RUN npm update rollup

COPY frontend/ /frontend
RUN npm run build

FROM debian:12-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y libpq5 ca-certificates curl

RUN useradd --system tendabike
USER tendabike
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/public"

COPY --from=build-engine /app/target/release/tendabike ./
COPY --from=build-frontend /frontend/dist/* ./dist/

ENTRYPOINT [ "./tendabike" ]
