# Release default: builds the book once at image-build time and serves the static output — no
# cargo/mdbook process running at container runtime. `compose.yaml`'s `docs` service overrides
# this image's `command` for the dev/watch profile instead of baking dev behavior in here: `mdbook
# serve` (paired with `develop.watch` syncing local edits in) is what actually gives live-reload,
# and that's a dev-only concern, not what this Dockerfile should run by default.
FROM rust:1-slim AS builder

RUN cargo install mdbook --locked --version "^0.4"

WORKDIR /book
COPY docs/ .
COPY spec/docs/ /spec/docs/
RUN mdbook build

FROM rust:1-slim

RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*

# The dev profile's `mdbook serve` override still needs mdbook and the unbuilt source present in
# the same image, so it isn't a separate build — carry both the built static output and the
# source/toolchain forward from the builder stage.
COPY --from=builder /root/.cargo/bin/mdbook /usr/local/bin/mdbook
COPY --from=builder /book /book
COPY --from=builder /spec/docs /spec/docs

WORKDIR /book

EXPOSE 3000
CMD ["python3", "-m", "http.server", "3000", "--directory", "dist/public", "--bind", "0.0.0.0"]
