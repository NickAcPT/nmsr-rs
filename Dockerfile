FROM rust:1.65 as builder

WORKDIR /usr/src/nmsr-aas
COPY . .

RUN apt-get install unzip
RUN cargo install --bin nmsr-aas --path .
RUN wget https://github.com/NickAcPT/nmsr-rs/releases/latest/download/parts.zip
RUN unzip parts.zip

CMD ["nmsr-aas"]

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
WORKDIR /nmsr-aas
COPY --from=builder /usr/local/cargo/bin/nmsr-aas /usr/local/bin/nmsr-aas
COPY --from=builder ./parts ./parts
COPY --from=builder ./example.config.toml ./config.toml

CMD ["nmsr-aas"]

EXPOSE 8080/tcp

