# Stage 1: Build the Vue/Vite frontend
FROM node:22-slim AS frontend

WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci

COPY index.html vite.config.js ./
COPY web ./web
RUN npm run build

# Stage 2: Analyse Rust dependencies with cargo-chef
FROM rust:1.95-slim AS chef

RUN apt-get update && apt-get install -y build-essential gcc-x86-64-linux-gnu pkg-config && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-gnu
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build the Rust binary
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
RUN cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-gnu

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY --from=frontend /app/dist ./dist

ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin hark

# Stage 4: Minimal runtime image
FROM cgr.dev/chainguard/glibc-dynamic

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/hark ./app
COPY --from=frontend /app/dist ./dist

EXPOSE 3000
CMD ["./app"]
