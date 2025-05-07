FROM rust:alpine AS base
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`

RUN apk add libpq-dev openssl-dev musl-dev openssl-libs-static
# hack libpq...
RUN <<EOF ar -M
open /usr/lib/libpq.a
addlib /usr/lib/libpgcommon_shlib.a
addlib /usr/lib/libpgcommon.a
addlib /usr/lib/libpgport.a
addlib /usr/lib/libssl.a
addlib /usr/lib/libcrypto.a
save
end
EOF

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
RUN cargo chef cook --release --recipe-path recipe.json


FROM cacher AS build-engine

# do not copy frontend!
COPY Cargo.toml Cargo.lock ./
COPY backend backend/

RUN cargo build --release

FROM node:slim AS build-frontend

WORKDIR /frontend

COPY frontend/package.json frontend/package-lock.json /frontend/
RUN npm update rollup

COPY frontend/ /frontend
RUN npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/dist"

COPY --from=build-engine /app/target/release/tendabike ./
COPY --from=build-frontend /frontend/dist dist

ENTRYPOINT [ "./tendabike" ]
