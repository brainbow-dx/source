# Release default: builds a real release binary at image-build time and runs that — no cargo
# process at container runtime. `compose.yaml`'s `relay` service (local dev only) instead builds
# the `builder` stage directly and overrides this image's `command` to `cargo run`, so bind-mounted
# source edits + the registry/target cache volumes it already has actually get used; this
# Dockerfile's own default is what a real deploy of this image should run.
FROM ubuntu:24.04 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain none

# atlas-relay is pure server code (axum/tokio), no wasm target needed. Pin a minimal toolchain
# directly rather than letting rustup read the workspace's own rust-toolchain.toml, which lists a
# much larger cross-platform target set meant for a developer's own machine — installing all of
# that at once inside a container has previously tripped `Input/output error` under Docker
# Desktop's virtiofs layer (see escher/runtimes/web/Dockerfile for the same fix, same root cause).
RUN rustup toolchain install stable --profile minimal
ENV RUSTUP_TOOLCHAIN=stable

# Skip sccache (set for local builds via .cargo/config.toml). Not worth it in a container.
ENV RUSTC_WRAPPER=""

WORKDIR /workspace
COPY . .
RUN cargo build --release --example serve -p atlas-relay

FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/examples/serve /usr/local/bin/atlas-relay

EXPOSE 9200

CMD ["atlas-relay", "0.0.0.0:9200"]
