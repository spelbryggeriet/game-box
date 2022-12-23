###############################################################################
# Backend building stage                                                      #
###############################################################################
FROM rust:1.66 as build-backend

# Install dependencies
RUN apt update
RUN apt install -y gcc-arm-linux-gnueabihf

# Create a new empty project for the backend
RUN USER=root cargo new --bin backend
WORKDIR /backend

# Create cargo config for setting the linker command
RUN mkdir ./.cargo
RUN echo "[target.armv7-unknown-linux-gnueabihf]\nlinker = \"arm-linux-gnueabihf-gcc\"" > ./.cargo/config

# Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./backend/Cargo.toml ./Cargo.toml

# Add target
RUN rustup target add armv7-unknown-linux-gnueabihf

# Build only the dependencies to cache them
RUN cargo build --release --target armv7-unknown-linux-gnueabihf
RUN rm src/*.rs

# Copy the source code
COPY ./backend/src ./src

# Build for release
RUN rm ./target/armv7-unknown-linux-gnueabihf/release/deps/game_box_backend*
RUN cargo build --release --target armv7-unknown-linux-gnueabihf

###############################################################################
# Frontend building stage                                                     #
###############################################################################
FROM rust:1.66 as build-frontend

# Install trunk command
RUN cargo install trunk@0.16.0

# Create a new empty project for the backend
RUN USER=root cargo new --bin frontend
WORKDIR /frontend

# Copy our manifests and index file
COPY ./Cargo.lock ./Cargo.lock
COPY ./frontend/Cargo.toml ./Cargo.toml
COPY ./frontend/index.html ./index.html

# Add target
RUN rustup target add wasm32-unknown-unknown

# Build only the dependencies to cache them
RUN trunk build --release
RUN rm src/*.rs

# Copy the source code
COPY ./frontend/src ./src

# Build for release
RUN rm ./target/wasm32-unknown-unknown/release/deps/game_box_frontend*
RUN trunk build --release

###############################################################################
# Final stage                                                                 #
###############################################################################
FROM debian:buster-slim

# Copy from the previous builds
COPY --from=build-backend /backend/target/armv7-unknown-linux-gnueabihf/release/game-box-backend /serve/game-box-backend
COPY --from=build-frontend /frontend/dist /serve/static

# Run the binary
ENV GAME_BOX_STATIC_DIR=/serve/static
CMD ["/serve/game-box-backend"]
