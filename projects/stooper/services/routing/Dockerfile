FROM nixos/nix:latest AS stooper-routing-base

RUN mkdir -p /etc/nix
RUN echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

RUN nix-channel --update
RUN nix-env -iA nixpkgs.rustc
RUN nix-env -iA nixpkgs.cargo
RUN nix-env -iA nixpkgs.deno

WORKDIR /workspace

CMD ["bash"]

FROM stooper-routing-base AS eden-dev

CMD ["bash"]
