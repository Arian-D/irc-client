# Stage 1: Builder
FROM rust:latest as builder

WORKDIR /app

# Install system dependencies for Linux desktop build
RUN apt-get install --update --yes libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libxdo-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

RUN cargo install tauri-cli
RUN cargo install trunk


# Add wasm32-unknown-unknown target for WASM compilation
RUN rustup target add wasm32-unknown-unknown

# Copy project files
# COPY . .

# Build frontend with Trunk
# RUN trunk build

# Build Tauri app
# RUN cd src-tauri && cargo build --release

WORKDIR /app

# Copy the built binary from builder
# COPY --from=builder /app/src-tauri/target/release/bundle/ /app/bundle/

# Set entrypoint to the built application
# Note: Adjust the binary name based on your actual output
# ENTRYPOINT ["/app/bundle/irc-client_0.1.0_amd64.deb"]
