FROM ubuntu:26.04
# Set to 2 to force Docker to bypass the cache
ARG CACHEBUST=1

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    sudo \
    && rm -rf /var/lib/apt/lists/*

ENV HOME=/home/ubuntu
USER ubuntu

# WORKAROUND: Pre-create Cargo cache directories so Docker named volumes
# automatically inherit the correct 'ubuntu' user ownership.
RUN mkdir -p /home/ubuntu/.cargo/registry /home/ubuntu/.cargo/git

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# WORKAROUND: Appease JetBrains Settings Sync
# The IDE expects the host's snap path. We symlink the container's binaries
# to /snap/bin so RustRover finds them automatically.
USER root
RUN mkdir -p /snap/bin \
    && ln -s $HOME/.cargo/bin/cargo /snap/bin/cargo \
    && ln -s $HOME/.cargo/bin/rustc /snap/bin/rustc \
    && ln -s $HOME/.cargo/bin/rustfmt /snap/bin/rustfmt \
    && ln -s $HOME/.cargo/bin/rustup /snap/bin/rustup
USER ubuntu

# Add Cargo to the PATH so the IDE and terminal can find it
ENV PATH="$HOME/.cargo/bin:$PATH"

# Trust the workspace directory to avoid Git ownership errors
RUN git config --global --add safe.directory /workspace

# Set the default working directory
WORKDIR /workspace