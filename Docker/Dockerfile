FROM rust:1.86.0-bookworm as builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    curl \
    libfontconfig1-dev \
    libfreetype6-dev \
    libexpat1-dev \
    libpng-dev \
    libjpeg-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release --example tx_loop
RUN apt-get update && apt-get install -y \
    binutils \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/deps /app/etc/ssl/certs
RUN ldd /app/target/release/examples/tx_loop | grep "=> /" | awk '{print $3}' | \
    xargs -I '{}' cp -L '{}' /app/deps/
RUN cp -L /etc/ssl/certs/ca-certificates.crt /app/etc/ssl/certs/

FROM gcr.io/distroless/cc-debian12

WORKDIR /app

COPY --from=builder /app/target/release/examples/tx_loop /app/
COPY --from=builder /app/deps/* /usr/lib/
COPY --from=builder /app/etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

CMD ["/app/tx_loop"]