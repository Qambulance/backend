FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/qambulance-backend .

ENV PORT=9000

EXPOSE 9000

CMD [ "./qambulance-backend" ]
