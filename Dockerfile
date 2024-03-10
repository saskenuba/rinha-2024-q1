FROM messense/rust-musl-cross:x86_64-musl as chef
WORKDIR /app

# USER root
RUN cargo install cargo-chef

FROM chef as planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json

COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl


FROM scratch as runtime

WORKDIR /usr/bin
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/main .

EXPOSE 8080
CMD ["./main"]
