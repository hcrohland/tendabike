FROM rust:alpine as build-engine

# hack libpq...
COPY patchlibpq ./
RUN apk add libpq-dev openssl-dev musl-dev openssl-libs-static
RUN ar -M < patchlibpq

# do not copy frontend!
COPY Cargo.toml Cargo.lock ./
COPY backend backend/

RUN cargo build --release

FROM node:21 AS build-frontend

WORKDIR /frontend

COPY frontend/package.json frontend/package-lock.json /frontend/
RUN npm update rollup

COPY frontend/ /frontend
RUN npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/dist"

COPY --from=build-engine /target/release/tendabike ./
COPY --from=build-frontend /frontend/dist dist

ENTRYPOINT [ "./tendabike" ]
