FROM rust:latest
WORKDIR /build
COPY . .
RUN cargo build --release
CMD ["/build/target/release/kvnode"]
