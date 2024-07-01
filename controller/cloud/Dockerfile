FROM rust:1.71.0 AS builder
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN apt-get update && apt-get install -y cmake libsqlite3-dev
RUN update-ca-certificates && apt update && apt install -y libssl-dev
WORKDIR /work
COPY ./ .
RUN rustup component add rustfmt && cargo build --release --bin cloud
FROM debian:11-slim
# 安装运行时所需的依赖项
RUN apt-get update && apt-get install -y \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /work/target/release/cloud /usr/local/bin/
CMD ["/usr/local/bin/cloud"]
