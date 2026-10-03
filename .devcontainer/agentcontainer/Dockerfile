# Development environment: Rust (stable) + Node 24 + npm + pi + gh
#
#   docker build -t rust-node-dev .
#
# Based on the devcontainers base image (Debian 13 trixie), which already
# provides git, curl, ca-certificates, sudo, less, zsh and a non-root user
# `vscode` (uid/gid 1000). Node comes from NodeSource since this image has
# no Node; Rust is layered on via rustup.

FROM mcr.microsoft.com/devcontainers/base:trixie

# System dependencies not already in the base image
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    tmux \
    vim \
    ripgrep \
    && rm -rf /var/lib/apt/lists/*

# Node 24 LTS (+ npm) from NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain, installed system-wide under /usr/local so every user gets it
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH="/usr/local/cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable \
    --component clippy \
    --component rustfmt \
    --component rust-src \
    && rustup -V

# Pi coding agent (https://pi.dev); --ignore-scripts is the documented
# install form. Global install puts `pi` on PATH for every user; per-user
# state lives in ~/.pi.
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

# GitHub CLI from GitHub's official apt repository. Per-user auth lives in
# ~/.config/gh (created on first `gh auth login`).
RUN mkdir -p /etc/apt/keyrings \
    && curl -fsSLo /etc/apt/keyrings/githubcli.gpg \
       https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli.gpg] https://cli.github.com/packages stable main" \
       > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*

# The base image ships a `vscode` user on uid/gid 1000 with passwordless
# sudo. Rename it to `pythia` (and its sudoers drop-in) rather than
# colliding on the UID. cargo/rustup homes are made world-writable so the
# non-root user can write the registry cache and run `rustup update`.
RUN groupmod -n pythia vscode \
    && usermod -l pythia -d /home/pythia -m -s /bin/bash vscode \
    && mv /etc/sudoers.d/vscode /etc/sudoers.d/pythia \
    && sed -i 's/vscode/pythia/g' /etc/sudoers.d/pythia \
    && chmod 0440 /etc/sudoers.d/pythia \
    && mkdir -p /workspaces \
    && chown pythia:pythia /workspaces \
    && chmod -R a+rwX /usr/local/cargo /usr/local/rustup

USER pythia
WORKDIR /workspaces

CMD ["bash"]