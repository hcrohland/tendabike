# Shared rust base: C linker deps + slim rustup profile. Both cargo
# stages inherit from it so these layers are built once and cached
# identically.
FROM rust:alpine AS rust-base
RUN apk add  musl-dev
RUN rustup set profile minimal

# Dependency stage: compiles the full dependency graph from the
# workspace manifests alone. Its cache key is a function of the
# manifests, Cargo.lock and the rust base image, so it stays warm
# across source-only commits. The compiled /app/target is a regular
# layer (not a cache mount), which is what lets the remote cache in
# .github/workflows/docker-image.yml (cache-from/cache-to type=gha)
# share it across runners. Workspace members are stubbed below so no
# real source feeds this stage; a lockfile change is the only thing
# that forces a full rebuild here.
FROM rust-base AS build-deps

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend/app/Cargo.toml backend/app/
COPY backend/domain/Cargo.toml backend/domain/
COPY backend/axum/Cargo.toml backend/axum/
COPY backend/strava/Cargo.toml backend/strava/
COPY backend/sqlx/Cargo.toml backend/sqlx/
COPY backend/exec/Cargo.toml backend/exec/
RUN mkdir -p backend/app/src \
             backend/domain/src/bin \
             backend/axum/src \
             backend/strava/src \
             backend/sqlx/src \
             backend/exec/src \
    && printf 'fn main() {}\n' > backend/app/src/main.rs \
    && printf 'fn main() {}\n' > backend/domain/src/bin/build_snapshot.rs \
    && for f in backend/domain/src/lib.rs \
                backend/axum/src/lib.rs \
                backend/strava/src/lib.rs \
                backend/sqlx/src/lib.rs \
                backend/exec/src/lib.rs; do \
        echo '// stub' > $f; \
    done
RUN cargo build --release

# Build stage: starts from the pre-compiled target directory, so cargo
# recompiles only the six real workspace crates.
FROM rust-base AS build-engine

WORKDIR /app
COPY --from=build-deps /app/target ./target

ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx/
COPY backend backend/
# The copied target directory carries cargo's fingerprints from the stub
# build: each unit's invoked.timestamp is the stub build time, which is
# always newer than the build-context mtimes that COPY preserves (the git
# checkout). Cargo's mtime check would then call every workspace source
# unchanged and ship the stub binary as-is. Touching the sources past
# those fingerprints forces the five workspace crates to recompile; the
# registry dependencies are untouched and stay cached.
RUN find backend -type f -name '*.rs' -exec touch {} + \
    && cargo build --release \
    && cp /app/target/release/tendabike /app/tendabike

FROM node:slim AS build-frontend

WORKDIR /build

COPY package.json package-lock.json /build/
COPY frontend/package.json /build/frontend/
RUN npm update rollup

COPY frontend/ /build/frontend
RUN npm run build

FROM scratch

USER 999:999
WORKDIR /tendabike
ENV STATIC_WWW="/tendabike/dist"

COPY --from=build-engine /app/tendabike ./
COPY --from=build-frontend /build/frontend/dist dist

ENTRYPOINT [ "./tendabike" ]
