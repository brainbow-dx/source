FROM nixos/nix:latest AS eden-psst-base

RUN mkdir -p /etc/nix
RUN echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

RUN nix-channel --update

RUN mkdir -p /workspace/brainbow/projects/eden/apps/psst
WORKDIR /workspace/brainbow/projects/eden/apps/psst

ENTRYPOINT ["scripts/entrypoint.sh"]

CMD ["bash"]

#---
FROM eden-psst-base AS eden-psst

CMD ["psst"]
