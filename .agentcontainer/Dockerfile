# Development environment: Rust (stable) + Node 24 + npm + pi
#
#   docker build -t rust-node-dev .
#   docker run --rm -it -v "$PWD":/home/pythia/src rust-node-dev
#
# The node:24 base (Debian 13 trixie) ships Node 24 LTS and npm; Rust is
# layered on top via rustup so the toolchain tracks upstream releases
# independently of the base image.

FROM node:24-trixie

# System dependencies:
#   build-essential  C toolchain (Rust needs a linker; some crates compile C)
#   pkg-config       for crates that link native libraries
#   git              cargo can fetch git dependencies
#   sudo             lets pythia run admin commands (see sudoers below)
#   curl / ca-certs  for rustup installer and the GitHub CLI repo key
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    pkg-config \
    sudo \
    tmux \
    ripgrep\
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain, installed system-wide under /usr/local so every user gets it
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH="/usr/local/cargo/bin:${PATH}"

# Minimal profile keeps the image lean; add commonly wanted dev components.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable \
    --component clippy \
    --component rustfmt \
    --component rust-src \
    && rustup -V

# Pi coding agent (https://pi.dev); --ignore-scripts is the documented
# install form (pi needs no lifecycle scripts). Global install puts `pi`
# on PATH for every user; per-user state lives in ~/.pi.
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

# GitHub CLI (https://cli.github.com) for `gh` (PRs, issues, Actions),
# installed from GitHub's official apt repository. The archive key is already
# a dearmored keyring, so it's fetched straight into /etc/apt/keyrings. Per-user
# auth lives in ~/.config/gh (created on first `gh auth login`).
RUN mkdir -p /etc/apt/keyrings \
    && curl -fsSLo /etc/apt/keyrings/githubcli.gpg \
    https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli.gpg] https://cli.github.com/packages stable main" \
    > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*

# The node:24 base image ships a `node` user already on uid/gid 1000, so
# rename it to `pythia` rather than collide on the UID; its home (which
# carries the pre-populated npm cache) moves to /home/pythia. Home doubles
# as the default workspace.
# cargo/rustup homes live under /usr/local, so make them world-writable:
# a single-user dev image, and the non-root user needs write access to the
# registry cache, package locks and toolchain dir (rustup update).
# pythia's login is passwordless (locked password inherited from node),
# so sudo is granted via an explicit NOPASSWD rule. Group membership alone
# won't do: the %sudo rule in /etc/sudoers requires a password and is
# evaluated before /etc/sudoers.d (first match wins).
RUN groupmod -n pythia node \
    && usermod -l pythia -d /home/pythia -m -s /bin/bash node \
    && echo "pythia ALL=(ALL:ALL) NOPASSWD: ALL" > /etc/sudoers.d/pythia \
    && chmod 0440 /etc/sudoers.d/pythia \
    && mkdir -p /home/pythia/src \
    && chown -R pythia:pythia /home/pythia \
    && chmod -R a+rwX /usr/local/cargo /usr/local/rustup

USER pythia
WORKDIR /home/pythia/src

CMD ["bash"]
