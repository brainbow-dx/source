# Cross-compiles a Rust binary for aarch64 Linux handhelds running muOS (Anbernic RG35XX-family
# and similar). Generic: knows nothing about any one project. `docker-build.sh` mounts a project's
# source into this image and runs `cargo build` inside it.
#
# `aarch64-unknown-linux-gnu`, not `-musl`: muOS's rootfs is glibc-based, and Bevy's audio/gamepad
# stack (`cpal`/`alsa-sys`, `gilrs`) links against real system libs (`libasound`, `libudev`) that
# only have well-maintained glibc packages on Debian/Ubuntu. A `-musl` static build would dodge the
# target-side `.so` question entirely but needs those libs' musl equivalents built by hand -- not
# worth it against a target we already know is glibc.
FROM rust:bookworm

# `gcc-aarch64-linux-gnu`: the actual cross linker/toolchain. `dpkg --add-architecture arm64` +
# the `:arm64` package suffix below pulls the *target*-architecture ALSA/udev headers and shared
# libs (not the host's own x86_64/arm64-host copies) so the cross linker has something real to link
# `cpal`'s `libasound` dependency and `gilrs`'s `libudev` dependency against. Without this, any
# build with escher-bevy's `audio` feature (or gamepad support) fails at link time with
# "cannot find -lasound" / "cannot find -ludev", not at compile time -- a real, easy-to-hit trap
# this image exists specifically to avoid.
RUN dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        gcc-aarch64-linux-gnu \
        libasound2-dev:arm64 \
        libudev-dev:arm64 \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig \
    PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig

WORKDIR /workspace
