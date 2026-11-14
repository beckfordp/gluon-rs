# SPDX-License-Identifier: MIT
# The Linux test bed's image (scripts/on-linux.sh): Linux with what the Rust
# repos' CI needs, as GitHub Actions has it (the services'
# .github/workflows/ci.yml) and gluon bin/sandbox-setup installs it.
FROM rust:1.99.1-bookworm

# rust-toolchain.toml's components: tester can't add them to root's rustup.
RUN rustup component add rustfmt clippy llvm-tools-preview

# PostgreSQL 16 binaries, from PGDG (bookworm has 15), for the throwaway
# clusters the tests start.
RUN install -d /usr/share/postgresql-common/pgdg \
 && curl -fsSL -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc \
      https://www.postgresql.org/media/keys/ACCC4CF8.asc \
 && echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      postgresql-16 rsync shellcheck python3 \
 && rm -rf /var/lib/apt/lists/*

# The same cargo-llvm-cov as CI and the sandbox; cargo-audit's latest, as CI's.
RUN cargo install --locked cargo-llvm-cov@0.9.1 cargo-audit \
 && rm -rf /usr/local/cargo/registry

# PostgreSQL refuses to run as root. /work holds the copied repos, CARGO_HOME
# cargo's downloads: both volumes, owned by tester.
RUN useradd -m -u 1000 tester \
 && install -d -o tester -g tester /work /home/tester/.cargo
ENV PATH=/usr/lib/postgresql/16/bin:$PATH CARGO_HOME=/home/tester/.cargo
USER tester
