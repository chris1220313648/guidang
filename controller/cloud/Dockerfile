FROM rust:1.71.1 AS builder
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN apt-get update && apt-get install -y cmake
RUN update-ca-certificates && apt update && apt install -y libssl-dev
WORKDIR /work
COPY ./ .
RUN rustup component add rustfmt && cargo build --release --bin cloud
FROM debian:11-slim
COPY --from=builder /work/target/release/cloud /usr/local/bin/
CMD ["/usr/local/bin/cloud"]
