FROM rust:1.59-slim-buster AS builder
WORKDIR /app
RUN cargo init peiji
WORKDIR /app/peiji
COPY Cargo.toml .
RUN cargo build --release
RUN rm Cargo.toml src/*
COPY . .
RUN cargo build --release


FROM gcr.io/distroless/cc
WORKDIR /app
COPY --from=builder /app/peiji/target/release/peiji .
CMD ["/app/peiji"]
