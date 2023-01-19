FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

ENV USER=means
ENV UID=10001

RUN adduser --disabled-password --gecos "" --home="/app" --shell "/sbin/nologin" --no-create-home --uid "${UID}" "${USER}"

WORKDIR /means

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /means

COPY --from=builder /means/target/x86_64-unknown-linux-musl/release/means ./

USER means:means

CMD ["/means/means"]
