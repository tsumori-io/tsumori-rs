# Specify the base image we're building from.
FROM rust:1.76-bookworm AS builder
WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config

# Build profile, release by default
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE $BUILD_PROFILE

# Extra Cargo flags
ARG RUSTFLAGS=""
ENV RUSTFLAGS "$RUSTFLAGS"

# Extra Cargo features
ARG FEATURES=""
ENV FEATURES $FEATURES

# Copy over dir.
COPY . .

# Run the build.
RUN cargo build --profile $BUILD_PROFILE --features "$FEATURES" --locked --bin tsumori

# ARG is not resolved in COPY so we have to hack around it by copying the
# binary to a temporary location
RUN cp /app/target/$BUILD_PROFILE/tsumori /app/tsumori

# Use Ubuntu as the release image
FROM debian:bookworm-slim as runtime
WORKDIR /app

# Copy tsumori over from the build stage
COPY --from=builder /app/tsumori /usr/local/bin

# # Copy licenses
# COPY LICENSE-* ./

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/tsumori"]
