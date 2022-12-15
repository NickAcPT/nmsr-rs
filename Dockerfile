FROM rustlang/rust:nightly-alpine as builder

RUN apk add --no-cache git musl-dev

WORKDIR /usr/src/nmsr-rs
RUN git clone https://github.com/NickAcPT/nmsr-rs .

RUN cargo +nightly -Z sparse-registry install --path nmsr-aas --bin nmsr-aas

RUN wget https://github.com/NickAcPT/nmsr-rs/releases/latest/download/parts.zip && \
    unzip parts.zip

FROM alpine:3.17.0

WORKDIR /nmsr-aas

COPY --from=builder /usr/local/cargo/bin/nmsr-aas /usr/local/bin/nmsr-aas
COPY --from=builder /usr/src/nmsr-rs/parts ./parts
RUN echo 'address = "0.0.0.0"' >> config.toml && \
    echo 'port = 8080' >> config.toml && \
    echo 'parts = "parts"' >> config.toml && \
    echo '[cache]' >> config.toml && \
    echo 'cleanup_interval = 3600' >> config.toml && \
    echo 'image_cache_expiry = 86400' >> config.toml && \
    echo 'mojang_profile_request_expiry = 900' >> config.toml && \
    echo 'mojang_profile_requests_per_second = 10' >> config.toml


EXPOSE 8080/tcp

CMD ["nmsr-aas", "-c", "/nmsr-aas/config.toml"]