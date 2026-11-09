### backend build

FROM rust:alpine AS rust-base
RUN apk add --no-cache musl-dev
RUN rustup set profile minimal
ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    CARGO_PROFILE_RELEASE_STRIP=symbols

FROM rust-base AS chef
RUN apk add --no-cache cargo-chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY backend backend/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-deps
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM build-deps AS build-engine
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY backend backend/
RUN cargo build --release \
    && cp target/release/tendabike /app/tendabike

### Frontend build
FROM node:slim AS build-frontend

WORKDIR /build

COPY package.json package-lock.json /build/
COPY frontend/package.json /build/frontend/
RUN npm ci

COPY frontend/ /build/frontend
RUN npm run build

### final assembly
FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/assets"

COPY --from=build-engine /app/tendabike ./
COPY --from=build-engine /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build-frontend /build/frontend/build assets

ENTRYPOINT [ "./tendabike" ]
