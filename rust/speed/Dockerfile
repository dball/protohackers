FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

ENV USER=speed
ENV UID=10001

RUN adduser --disabled-password --gecos "" --home="/app" --shell "/sbin/nologin" --no-create-home --uid "${UID}" "${USER}"

WORKDIR /speed

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /speed

COPY --from=builder /speed/target/x86_64-unknown-linux-musl/release/speed ./

USER speed:speed

CMD ["/speed/speed"]
