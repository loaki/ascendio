# notfl3/cargo-apk ships Rust 1.79, too old for current crates (fontdue needs
# 1.87), and only the android-31 platform, old enough that Play Protect warns
# on install. Its NDK r25 is fine with current Rust, so only those change.
FROM notfl3/cargo-apk
RUN rustup toolchain install stable --profile minimal \
        --target aarch64-linux-android armv7-linux-androideabi \
    && rustup default stable
RUN curl -sfLo /tmp/p.zip https://dl.google.com/android/repository/platform-35_r02.zip \
    && unzip -q /tmp/p.zip -d $ANDROID_HOME/platforms && rm /tmp/p.zip
