FROM nixos/nix:latest AS eden-base

RUN mkdir -p /etc/nix
RUN echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

RUN nix-channel --update
RUN nix-env -iA nixpkgs.git
RUN nix-env -iA nixpkgs.vim
RUN nix-env -iA nixpkgs.htop
RUN nix-env -iA nixpkgs.rustc
RUN nix-env -iA nixpkgs.cargo
RUN nix-env -iA nixpkgs.cachix
RUN nix-env -iA nixpkgs.deno

WORKDIR /workspace

CMD ["bash"]

FROM eden-base AS eden-dev

CMD ["bash"]
