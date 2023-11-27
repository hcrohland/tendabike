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

FROM gcr.io/distroless/base-debian12
COPY --from=build-engine /lib/aarch64-linux-gnu/libcrypto.so.3 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libgssapi_krb5.so.2 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libldap-2.5.so.0 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libkrb5.so.3 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libk5crypto.so.3 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libcom_err.so.2 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libkrb5support.so.0 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/liblber-2.5.so.0 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libsasl2.so.2 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libgnutls.so.30 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libkeyutils.so.1 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libresolv.so.2 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libp11-kit.so.0 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libidn2.so.0 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libunistring.so.2 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libtasn1.so.6 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libnettle.so.8 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libhogweed.so.6 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libgmp.so.10 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libffi.so.8 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libgcc_s.so.1 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libssl.so.3 /lib/aarch64-linux-gnu/
COPY --from=build-engine /lib/aarch64-linux-gnu/libpq.so.5 /lib/aarch64-linux-gnu/

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/public"

COPY --from=build-engine /app/target/release/tendabike ./
COPY --from=build-frontend /frontend/public/* ./public/

ENTRYPOINT [ "./tendabike" ]
