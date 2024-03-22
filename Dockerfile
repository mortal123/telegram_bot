FROM rust:1.77 as builder

WORKDIR /repo

COPY . .
RUN cargo build -p telegram-bot --release

FROM debian:11

COPY --from=builder /repo/target/release/telegram-bot /usr/local/bin/

CMD ["telegram-bot"]
