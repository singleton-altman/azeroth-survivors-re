# Docker build for the browser (wasm) version of azeroth-survivors.
#
# The image contains only build artifacts: the wenilla-host server binary and the
# web/dist bundle. The WoW 1.12.1 game data (~4 GB of MPQs) is NOT baked in — fetch
# it on the host with scripts/fetch-data.sh and bind-mount it at runtime, so image
# rebuilds never re-download it. See docker-compose.yml for the runtime wiring.
#
# Build:  docker build -t azeroth-survivors .
# Run:    docker run --rm -p 127.0.0.1:8090:8090 -v "$PWD/wow-data:/app/wow-data:ro" azeroth-survivors

FROM rust:1.98.1-bookworm AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends brotli curl unzip ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# rustup picks the pinned 1.98.1 toolchain from rust-toolchain.toml automatically.
# USTC mirror for the wasm32 std component: static.rust-lang.org throttles to
# ~100 KB/s per connection on some networks, and TUNA's mirror lags the
# manifest format rustup expects ("obsolete distribution manifest").
ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static \
    RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup

# Prefer IPv4 on dual-stack hosts: several CDNs (release-assets.githubusercontent.com,
# archive.org) throttle or blackhole over IPv6 on this network class.
RUN echo 'precedence ::ffff:0:0/96  100' >> /etc/gai.conf

# All-foreign-mirrors setup so the build never touches a direct international
# route (throttled/blackholed here): crates.io via ByteDance's rsproxy, rustup
# dist via USTC (above), GitHub release assets via a gh-proxy frontend.
# GH_PROXY can be overridden with --build-arg GH_PROXY= (empty = direct).
ARG GH_PROXY=https://gh-proxy.com/
ENV CARGO_HOME=/usr/local/cargo
RUN printf '[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy-sparse]\nregistry = "sparse+https://rsproxy.cn/index/"\n\n[net]\ngit-fetch-with-cli = true\n' > "${CARGO_HOME}/config.toml"

# Fetch the two binary toolchains via the proxy with IPv4 forced; web-setup.sh
# detects them and skips its own downloads.
ARG TARGETARCH
RUN case "${TARGETARCH}" in \
      arm64) wasi_plat=arm64-linux; binen_plat=aarch64-linux ;; \
      amd64) wasi_plat=x86_64-linux; binen_plat=x86_64-linux ;; \
      *) echo "unsupported TARGETARCH ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
 && mkdir -p tools/wasi-sdk tools/binaryen \
 && curl -4 -fsSL --retry 3 "${GH_PROXY}https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-27/wasi-sdk-27.0-${wasi_plat}.tar.gz" | tar xz --strip-components=1 -C tools/wasi-sdk \
 && curl -4 -fsSL --retry 3 "${GH_PROXY}https://github.com/WebAssembly/binaryen/releases/download/version_130/binaryen-version_130-${binen_plat}.tar.gz" | tar xz --strip-components=1 -C tools/binaryen

# web-setup.sh: wasm32 target, wasm-bindgen-cli (version pinned by Cargo.lock),
# wasi-sdk (Lua VM sysroot), binaryen — all with Linux-aarch64/Linux-x86_64 releases.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    ./scripts/web-setup.sh

# The wasm client. WEB_BACKEND is webgpu by default; build with
# --build-arg WEB_BACKEND=webgl2 for a WebGL2 build (glue screens only).
ARG WEB_BACKEND=webgpu
ENV WEB_BACKEND=${WEB_BACKEND}

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    ./scripts/web-build.sh

# wenilla-host (the static file + game-data + websocket-proxy server).
# Artifacts are copied to /out because cache mounts are not part of the image layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p wenilla-host \
 && mkdir -p /out \
 && cp target/release/wenilla-host /out/wenilla-host

FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /out/wenilla-host /app/wenilla-host
COPY --from=builder /app/web/dist /app/web/dist

# wenilla-host is a local-testing tool: /data serves game files with no login.
# Binding 0.0.0.0 inside the container is required for port mapping; publish the
# port to host loopback only (-p 127.0.0.1:8090:8090) to keep that property.
EXPOSE 8090
ENTRYPOINT ["/app/wenilla-host"]
CMD ["--bind", "0.0.0.0:8090", "--www", "/app/web/dist", "--data", "/app/wow-data/Data"]
