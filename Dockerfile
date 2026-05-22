# Multi-stage build for Rust API with cargo-chef
FROM rust:1.95-slim AS chef

RUN apt-get update && apt-get install -y curl build-essential gcc-x86-64-linux-gnu pkg-config && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-gnu
RUN cargo install cargo-chef

# Download the standalone binary (ensure you pick the right version/arch)
RUN curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v4.3.0/tailwindcss-linux-x64 \
    && chmod +x tailwindcss-linux-x64 \
    && mv tailwindcss-linux-x64 /usr/local/bin/tailwindcss

WORKDIR /app

# Planner stage - analyze dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - cache dependencies
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies only (this layer is cached)
ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
RUN cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-gnu

# Copy source and build application
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates

ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin hark

# Runtime stage
FROM cgr.dev/chainguard/glibc-dynamic

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/hark ./app

EXPOSE 3000
CMD ["./app"]
