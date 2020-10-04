# Use latest because we need nightly
FROM rust:latest AS build-engine

ENV DEBIAN_FRONTEND=noninteractive
# install nighlty toolchain
RUN rustup update nightly && rustup default nightly

# install dependencies
RUN apt-get update && apt-get install -y libpq-dev libssl-dev

COPY ./ ./

RUN cargo build --release 

RUN mkdir -p /build-out

RUN cp target/release/tb_engine Rocket.toml /build-out


FROM node:12-buster AS build-frontend

COPY tb_svelte ./

WORKDIR /tb_svelte

RUN npm install && npm run build

RUN mkdir -p /build-out
RUN cp -R public /build-out

FROM debian:buster

ENV DEBIAN_FRONTED=noninteractive

RUN apt-get update && apt-get install -y libpq-dev libssl-dev

COPY --from=build-engine /build-out/* /
COPY --from=build-frontend /build-out/* /

USER tendabike

ENTRYPOINT [ "/tb_engine" ]
