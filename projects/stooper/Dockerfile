FROM nixos/nix AS stooper-base

RUN mkdir -p /etc/nix
RUN echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

RUN nix-channel --update

WORKDIR /workspace

CMD ["bash"]

FROM stooper-base AS stooper-dev

RUN nix-env -iA \
    nixpkgs.cachix nixpkgs.htop nixpkgs.vim \
    nixpkgs.rustc nixpkgs.cargo \
    nixpkgs.deno

CMD ["bash"]
