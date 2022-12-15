FROM rustlang/rust:nightly-alpine as builder

ENV CACHE_CLEANUP_INTERVAL = 3600
ENV CACHE_IMAGE_CACHE_EXPIRY = 86400
ENV CACHE_MOJANG_PROFILE_REQUEST_EXPIRY = 900
ENV CACHE_MOJANG_PROFILE_REQUESTS_PER_SECOND = 10


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
    echo 'cleanup_interval = $CACHE_CLEANUP_INTERVAL' >> config.toml && \
    echo 'image_cache_expiry = $CACHE_IMAGE_CACHE_EXPIRY' >> config.toml && \
    echo 'mojang_profile_request_expiry = $CACHE_MOJANG_PROFILE_REQUEST_EXPIRY' >> config.toml && \
    echo 'mojang_profile_requests_per_second = $CACHE_MOJANG_PROFILE_REQUESTS_PER_SECOND' >> config.toml


EXPOSE 8080/tcp

CMD ["nmsr-aas", "-c", "/nmsr-aas/config.toml"]