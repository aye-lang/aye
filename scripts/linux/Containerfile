# Ubuntu 24.04 development image for Aye: scripts/linux.sh builds and runs it.
# RUSTUP_HOME and CARGO_HOME must match scripts/setup-ubuntu.sh.
FROM ubuntu:24.04
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo SDL_VIDEODRIVER=dummy
COPY scripts/setup-ubuntu.sh /tmp/setup-ubuntu.sh
RUN sh /tmp/setup-ubuntu.sh && rm /tmp/setup-ubuntu.sh
WORKDIR /work
