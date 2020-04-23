FROM rust:1.42 AS builder

WORKDIR /src/
RUN USER=root cargo new application-operator
WORKDIR /src/application-operator
COPY Cargo.toml Cargo.lock ./
RUN cargo build --locked
COPY src src/
RUN cargo build --release --locked

# move to scratch at some point
FROM debian:buster

COPY --from=builder /src/application-operator/target/release/application-operator /bin/application-operator

CMD ["/bin/application-operator"]
