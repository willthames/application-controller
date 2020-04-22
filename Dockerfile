FROM rust:1.42 AS builder

WORKDIR /src/
RUN USER=root cargo new application-operator
WORKDIR /src/application-operator
COPY Cargo.toml Cargo.lock ./
RUN cargo update
COPY src src/
RUN cargo build --release

# move to scratch at some point
FROM debian:buster
RUN apt-get update && apt install -y libssl-dev

COPY --from=builder /src/application-operator/target/release/application-operator /bin/application-operator

CMD ["/bin/application-operator"]
