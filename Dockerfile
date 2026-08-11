FROM ubuntu:24.04 AS dev
# FROM lscr.io/linuxserver/code-server:latest AS dev

RUN apt-get update && apt-get install -y curl git unzip

# ENV RUSTUP_HOME=/usr/local/rustup
# ENV CARGO_HOME=/usr/local/cargo
# ENV PATH=/usr/local/cargo/bin:$PATH
# RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rust-install.sh
RUN sh rust-install.sh -y --no-modify-path

# ENV DENO_INSTALL=/usr/local
RUN curl -fsSL https://deno.land/install.sh -o deno-install.sh
RUN sh deno-install.sh -y

WORKDIR /workspace

# RUN curl -fsSL https://code-server.dev/install.sh -o code-server-install.sh
# RUN sh code-server-install.sh

# TODO: Move this stuff to a bootstrap.sh script and mount it.
# RUN code-server --install-extension denoland.vscode-deno
# RUN code-server --install-extension rust-lang.rust-analyzer
# RUN code-server --install-extension ./projects/escher/extensions/vscode/.output/pkg/brainbow.escher.vsix

CMD ["sleep", "infinity"]