FROM mcr.microsoft.com/devcontainers/typescript-node

ADD --checksum=sha256:8636e693ea0e05f5f4294b161f816c4d8df065267fdb0405cfb84c8e326991fa \
  https://github.com/astral-sh/uv/releases/download/0.9.27/uv-x86_64-unknown-linux-gnu.tar.gz \
  /home/node/.local/bin/uv.tgz

ADD --checksum=sha256:20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c \
  https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init \
  /home/node/.local/bin/rustup-init

RUN set -x \
 && cd /home/node/.local/bin \
 && tar -xf uv.tgz --strip-components=1 && rm uv.tgz \
 && chmod 755 rustup-init \
 && chown -R node.node /home/node/.local
