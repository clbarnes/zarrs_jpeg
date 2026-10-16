test-rust:
    cargo test --no-default-features --features zarrs,backend-rust

test-turbojpeg:
    cargo test --no-default-features --features zarrs,backend-turbojpeg

test-all: test-turbojpeg test-rust
