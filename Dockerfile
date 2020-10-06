FROM tendabike/build AS build-engine

ENV DEBIAN_FRONTEND=noninteractive

COPY ./ ./

RUN cd tb_engine &&  diesel migration run || true
RUN cd tb_strava &&  diesel migration run || true

RUN cargo build --release

RUN mkdir -p /build-out

RUN cp target/release/tb_engine /build-out


FROM node:12-buster AS build-frontend

COPY tb_svelte/ /tb_svelte

WORKDIR /tb_svelte

RUN npm install

RUN npm run build

RUN mkdir -p /build-out
RUN cp -R public /build-out

FROM debian:buster

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y libpq5 libssl1.1 ca-certificates

RUN useradd --system tendabike
USER tendabike
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/public"

COPY --from=build-engine /build-out/* ./
COPY --from=build-frontend /build-out/* ./public/

ENTRYPOINT [ "./tb_engine" ]