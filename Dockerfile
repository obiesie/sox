FROM rust:1.75

WORKDIR /usr/src/sox

COPY . .

RUN cargo build

CMD ["./target/debug/sox", "resources/main.sox"]