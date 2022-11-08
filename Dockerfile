FROM rust:alpine AS build
WORKDIR /build
COPY servus servus
COPY Cargo.lock .
COPY Cargo.toml .
RUN apk add musl-dev
RUN cargo build --release

FROM alpine
COPY --from=build /build/target/release/servus /bin/servus
ENV SERVUS_ADDRESS="0.0.0.0:80"
EXPOSE 80
ENTRYPOINT [ "/bin/servus" ]