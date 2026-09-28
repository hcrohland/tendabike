FROM rust:alpine AS build-engine
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN apk add  musl-dev

WORKDIR /app
# install nighlty toolchain
RUN rustup set profile minimal

ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx/
COPY backend backend/

# Cache mounts keep cargo's registry/git checkouts and compiled artifacts
# warm across builds; the CI workflow persists them via the GHA cache
# (cache-to: type=gha,mode=max in .github/workflows/docker-image.yml).
# Mount only the registry/git subtrees, never all of CARGO_HOME: the rust
# image's cargo/rustc shims live in $CARGO_HOME/bin and would be shadowed.
# Mount contents never land in the image, so the binary is copied out of
# the target mount before it unmounts.
RUN --mount=type=cache,id=rust-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rust-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rust-target,target=/app/target \
    cargo build --release \
    && cp /app/target/release/tendabike /app/tendabike-bin

FROM node:slim AS build-frontend

WORKDIR /build

COPY package.json package-lock.json /build/
COPY frontend/package.json frontend/package-lock.json /build/frontend/
RUN npm update rollup

COPY frontend/ /build/frontend
RUN npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/dist"

COPY --from=build-engine /app/tendabike-bin ./
COPY --from=build-frontend /build/frontend/dist dist

ENTRYPOINT [ "./tendabike" ]
