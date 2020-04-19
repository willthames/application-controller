FROM rust:1.42

WORKDIR /src/
RUN USER=root cargo new application-operator
WORKDIR /src/application-operator
COPY Cargo.toml Cargo.lock ./
RUN cargo update
COPY src src/
RUN cargo install --path . --root /bin

CMD ["/bin/application-operator"]
