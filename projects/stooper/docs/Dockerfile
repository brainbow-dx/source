# Dev-mode docs server — installs mdbook once at image build time, then serves the book at
# runtime. `compose.yaml`'s `docs` service pairs this with `develop.watch`, which syncs local
# edits straight into the running container; `mdbook serve`'s own file watcher picks those up
# and rebuilds + live-reloads (via its built-in websocket) with no container restart needed.
FROM rust:1-slim

RUN cargo install mdbook --locked --version "^0.4"

WORKDIR /book
COPY docs/ .

EXPOSE 3000
CMD ["mdbook", "serve", "--hostname", "0.0.0.0", "--port", "3000"]
