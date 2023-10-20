FROM --platform=$BUILDPLATFORM node:20 AS build-frontend

WORKDIR /frontend

COPY frontend/package.json frontend/package-lock.json /frontend/
RUN npm update rollup

COPY frontend/ /frontend
RUN npm run build

FROM debian:12-slim
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y libpq5 libssl3 ca-certificates curl

RUN useradd --system tendabike
USER tendabike
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/public"
ARG TARGET=""

COPY tendabike ./tendabike
COPY --from=build-frontend /frontend/public/* ./public/

ENTRYPOINT [ "./tendabike" ]