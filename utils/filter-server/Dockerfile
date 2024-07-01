FROM rust:1.71.0 AS builder
RUN update-ca-certificates
WORKDIR /work
COPY ./ .
RUN cargo build --release --bin filter-server
FROM debian:11-slim
COPY --from=builder /work/target/release/filter-server /usr/local/bin/
CMD ["/usr/local/bin/filter-server"]
