ARG RUST_VERSION=1.78.0

FROM rust:${RUST_VERSION} as builder

WORKDIR /app
COPY . /app

RUN cargo build --release
RUN strip --strip-unneeded target/release/masked-mails

FROM gcr.io/distroless/cc-debian12

USER nonroot

COPY --from=builder /app/target/release/masked-mails /

ENTRYPOINT ["/masked-mails"]
