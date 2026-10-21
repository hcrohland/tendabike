# Development environment: Rust (stable) + Node 24 + npm + opencode
#
#   docker build -t rust-node-dev .
#
# Based on the devcontainers base image (Debian 13 trixie), which already
# provides git, curl, ca-certificates, sudo, less, zsh and a non-root user
# `vscode` (uid/gid 1000). Node comes from NodeSource since this image has
# no Node; Rust is layered on via rustup.

FROM mcr.microsoft.com/devcontainers/base:trixie

# System dependencies not already in the base image. golang-go is needed to
# build mcp-language-server (see below).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    golang-go \
    tmux \
    vim \
    ripgrep \
    && rm -rf /var/lib/apt/lists/*

RUN cat >> /etc/bash.bashrc <<'EOF'

# Start a per-container ssh-agent on first interactive shell, reuse it after
case $- in *i*)
  export SSH_AUTH_SOCK=/tmp/ssh-agent-$(id -un).sock
  ssh-add -l >/dev/null 2>&1
  if [ $? -eq 2 ]; then
    rm -f "$SSH_AUTH_SOCK"
    ssh-agent -a "$SSH_AUTH_SOCK" >/dev/null
  fi
;; esac
EOF

# Node 24 LTS (+ npm) from NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Language servers (LSPs) for the agent, installed globally so
# mcp-language-server can find them on PATH:
#   - typescript-language-server + typescript (TypeScript/JS)
#   - svelte-language-server (exposes the `svelteserver` binary)
RUN npm install -g --ignore-scripts \
    typescript \
    typescript-language-server \
    svelte-language-server

# Rust toolchain, installed system-wide under /usr/local so every user gets it
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH="/usr/local/cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable \
    --component clippy \
    --component rustfmt \
    --component rust-src \
    --component rust-analyzer \
    && rustup -V

RUN cargo install sqlx-cli --no-default-features --features postgres,rustls

# The base image ships a `vscode` user on uid/gid 1000 with passwordless
# sudo. Rename it to `laios` (and its sudoers drop-in) rather than
# colliding on the UID. cargo/rustup homes are made world-writable so the
# non-root user can write the registry cache and run `rustup update`.
RUN groupmod -n laios vscode \
    && usermod -l laios -d /home/laios -m -s /bin/bash vscode \
    && mv /etc/sudoers.d/vscode /etc/sudoers.d/laios \
    && sed -i 's/vscode/laios/g' /etc/sudoers.d/laios \
    && chmod 0440 /etc/sudoers.d/laios \
    && mkdir -p /workspaces \
    && chown laios:laios /workspaces \
    && chmod -R a+rwX /usr/local/cargo /usr/local/rustup

USER laios
LABEL devcontainer.metadata='[{"remoteUser":"laios"}]'

WORKDIR /workspaces

# OpenCode (https://opencode.ai). The official install script drops the binary
# in the user's home (~/.opencode/bin); being user-owned, the `laios` user can
# upgrade it later by re-running the same command, no root needed.
# --no-modify-path keeps it out of the shell rc files, so we own the PATH entry
# below (reliable for both login and non-login shells).
RUN curl -fsSL https://opencode.ai/v2/install | bash -s -- --no-modify-path
ENV PATH="/home/laios/.opencode/bin:${PATH}"

# mcp-language-server: the MCP wrapper that exposes the language servers above
# to the agent. It is a Go binary, so we `go install` it as the `laios` user;
# the binary lands in ~/go/bin, matching the path in the agent's OpenCode
# config.
RUN go install github.com/isaacphi/mcp-language-server@latest
ENV PATH="/home/laios/go/bin:${PATH}"

RUN git config --global core.hooksPath .githooks

CMD ["bash"]