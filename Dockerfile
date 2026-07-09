# syntax=docker/dockerfile:1

# ---- Stage 1: build the SPA -------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /web
# Install deps from the lockfile first for better layer caching.
COPY web/package.json web/package-lock.json ./
RUN npm ci
# Build the SPA (tsc typecheck + vite build) into /web/dist.
COPY web/ ./
RUN npm run build

# ---- Stage 2: build the API binary -----------------------------------------
FROM rust:1-bookworm AS api
WORKDIR /src
# Copy the whole workspace (Cargo.lock pins the dependency graph) and build.
COPY Cargo.lock Cargo.toml ./
COPY domain ./domain
COPY application ./application
COPY infrastructure ./infrastructure
COPY api ./api
RUN cargo build --release -p tasklist-api

# ---- Stage 3: runtime image ------------------------------------------------
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# The API defaults to binding 0.0.0.0:8080 and serving the SPA from web/dist
# (relative to the working directory).
ENV TASKLIST_ENV=cloud \
    TASKLIST_BIND_ADDRESS=0.0.0.0:8080 \
    TASKLIST_WEB_DIST=web/dist
COPY --from=api /src/target/release/tasklist-api ./tasklist-api
COPY --from=web /web/dist ./web/dist
EXPOSE 8080
CMD ["./tasklist-api"]
