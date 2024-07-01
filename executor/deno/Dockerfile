FROM rust:1.71.0 AS builder
RUN apt-get update && apt-get install -y cmake
RUN update-ca-certificates && apt update && apt install -y libssl-dev
WORKDIR /work
COPY ./ .
RUN rustup component add rustfmt && cargo build --release --bin deno_executor
FROM debian:11-slim
COPY --from=builder /work/target/release/deno_executor /usr/local/bin/
CMD ["/usr/local/bin/deno_executor"]
