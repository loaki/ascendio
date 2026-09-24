# notfl3/cargo-apk ships Rust 1.79, too old for current crates (fontdue needs
# 1.87). Its NDK r25 is fine with current Rust, so only the toolchain changes.
FROM notfl3/cargo-apk
RUN rustup toolchain install stable --profile minimal \
        --target aarch64-linux-android armv7-linux-androideabi \
    && rustup default stable
