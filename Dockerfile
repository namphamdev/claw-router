# ── Stage 1: Build frontend ──────────────────────────────────────────
FROM node:22-alpine AS frontend-build

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ── Stage 2: Build backend ──────────────────────────────────────────
FROM rust:1.87-bookworm AS backend-build

WORKDIR /app/backend
# Cache dependencies by building a dummy project first
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

# Build real source
COPY backend/src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

# ── Stage 3: Runtime ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=backend-build /app/backend/target/release/backend ./backend

# Copy frontend build output into static/ (served by the backend)
COPY --from=frontend-build /app/frontend/dist ./static

EXPOSE 3000

CMD ["./backend"]
