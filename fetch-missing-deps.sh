#!/bin/bash
set -euo pipefail

SYSROOT="$HOME/msys2-sysroot"
REPO="https://repo.msys2.org/mingw/mingw64"
CACHE="$SYSROOT/.cache"
INDEX="$CACHE/index.html"

download_and_extract() {
    local pkg_prefix="$1"
    local filename=$(grep -oP "href=\"${pkg_prefix}-[0-9][^\"]*\.pkg\.tar\.zst\"" "$INDEX" \
        | sed 's/href="//;s/"//' \
        | sort -V \
        | tail -1)

    if [ -z "$filename" ]; then
        echo "WARNING: Could not find $pkg_prefix"
        return 1
    fi

    local cached="$CACHE/$filename"
    if [ ! -f "$cached" ]; then
        echo "Downloading $filename..."
        curl -sL "$REPO/$filename" -o "$cached"
    else
        echo "Cached: $filename"
    fi
    zstd -dq "$cached" -o "$cached.tar" --force 2>/dev/null
    tar -C "$SYSROOT" -xf "$cached.tar" 2>/dev/null || true
    rm -f "$cached.tar"
}

# Missing transitive dependencies
EXTRA_PACKAGES=(
    mingw-w64-x86_64-graphite2
    mingw-w64-x86_64-vulkan-loader
    mingw-w64-x86_64-vulkan-headers
    mingw-w64-x86_64-libsysprof-capture
    mingw-w64-x86_64-gst-plugins-bad
    mingw-w64-x86_64-pango
    mingw-w64-x86_64-libdatrie
    mingw-w64-x86_64-libthai
    mingw-w64-x86_64-libcloudproviders
    mingw-w64-x86_64-tracker3
    mingw-w64-x86_64-json-glib
    mingw-w64-x86_64-glib-networking
    mingw-w64-x86_64-openssl
    mingw-w64-x86_64-c-ares
    mingw-w64-x86_64-nghttp3
    mingw-w64-x86_64-ngtcp2
    mingw-w64-x86_64-libpsl
    mingw-w64-x86_64-libunistring
    mingw-w64-x86_64-libidn2
    mingw-w64-x86_64-wineditline
    mingw-w64-x86_64-readline
    mingw-w64-x86_64-termcap
    mingw-w64-x86_64-icu
)

for pkg in "${EXTRA_PACKAGES[@]}"; do
    download_and_extract "$pkg" || true
done

echo ""
echo "Testing pkg-config resolution..."
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_LIBDIR="$HOME/msys2-sysroot/mingw64/lib/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$HOME/msys2-sysroot"

for lib in gtk4 libadwaita-1 gstreamer-1.0 gstreamer-play-1.0 cairo glib-2.0; do
    if pkg-config --exists "$lib" 2>/dev/null; then
        echo "  ✓ $lib"
    else
        echo "  ✗ $lib — $(pkg-config --print-errors "$lib" 2>&1 | head -1)"
    fi
done
