flamegraph:
  export CARGO_PROFILE_RELEASE_DEBUG=true; cargo +nightly flamegraph

bench:
  cargo +nightly bench
