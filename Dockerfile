FROM messense/rust-musl-cross:x86_64-musl as chef

USER root
RUN cargo install cargo-chef

FROM chef as base

ENV APP_DIR=/usr/run/app
COPY src ${APP_DIR}/src
COPY benches ${APP_DIR}/benches
COPY "Cargo.toml" \
     "Cargo.lock" \
     ${APP_DIR}/

WORKDIR ${APP_DIR}

FROM base as planner
RUN cargo chef prepare --recipe-path recipe.json

FROM base as builder
COPY --from=planner ${APP_DIR}/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
RUN cargo build --release --target x86_64-unknown-linux-musl --target-dir ${APP_DIR}/target

FROM scratch as runtime
EXPOSE 80
COPY --from=builder /usr/run/app/target/x86_64-unknown-linux-musl/release/main .
CMD ["./main"]