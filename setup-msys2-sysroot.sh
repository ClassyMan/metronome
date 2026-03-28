#!/bin/bash
set -euo pipefail

SYSROOT="$HOME/msys2-sysroot"
REPO="https://repo.msys2.org/mingw/mingw64"
CACHE="$SYSROOT/.cache"

mkdir -p "$SYSROOT" "$CACHE"

# Fetch the full file listing once
INDEX="$CACHE/index.html"
if [ ! -f "$INDEX" ] || [ "$(find "$INDEX" -mmin +60 2>/dev/null)" ]; then
    echo "Fetching package index..."
    curl -sL "$REPO/" -o "$INDEX"
fi

download_and_extract() {
    local pkg_prefix="$1"
    # Find the latest version of this package from the index
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

    echo "  Extracting..."
    zstd -dq "$cached" -o "$cached.tar" --force 2>/dev/null
    tar -C "$SYSROOT" -xf "$cached.tar" 2>/dev/null || true
    rm -f "$cached.tar"
}

PACKAGES=(
    mingw-w64-x86_64-glib2
    mingw-w64-x86_64-gettext-runtime
    mingw-w64-x86_64-libiconv
    mingw-w64-x86_64-libffi
    mingw-w64-x86_64-pcre2
    mingw-w64-x86_64-zlib
    mingw-w64-x86_64-gtk4
    mingw-w64-x86_64-cairo
    mingw-w64-x86_64-pango
    mingw-w64-x86_64-gdk-pixbuf2
    mingw-w64-x86_64-graphene
    mingw-w64-x86_64-harfbuzz
    mingw-w64-x86_64-fribidi
    mingw-w64-x86_64-fontconfig
    mingw-w64-x86_64-freetype
    mingw-w64-x86_64-libpng
    mingw-w64-x86_64-libjpeg-turbo
    mingw-w64-x86_64-libtiff
    mingw-w64-x86_64-pixman
    mingw-w64-x86_64-libepoxy
    mingw-w64-x86_64-libxml2
    mingw-w64-x86_64-libadwaita
    mingw-w64-x86_64-gstreamer
    mingw-w64-x86_64-gst-plugins-base
    mingw-w64-x86_64-gst-plugins-good
    mingw-w64-x86_64-libvorbis
    mingw-w64-x86_64-libogg
    mingw-w64-x86_64-gobject-introspection-runtime
    mingw-w64-x86_64-appstream-glib
    mingw-w64-x86_64-sassc
    mingw-w64-x86_64-libsass
    mingw-w64-x86_64-expat
    mingw-w64-x86_64-brotli
    mingw-w64-x86_64-libdeflate
    mingw-w64-x86_64-lerc
    mingw-w64-x86_64-xz
    mingw-w64-x86_64-zstd
    mingw-w64-x86_64-libwebp
    mingw-w64-x86_64-orc
    mingw-w64-x86_64-opus
    mingw-w64-x86_64-flac
    mingw-w64-x86_64-libsndfile
    mingw-w64-x86_64-mpg123
    mingw-w64-x86_64-lame
    mingw-w64-x86_64-libsoup3
    mingw-w64-x86_64-sqlite3
    mingw-w64-x86_64-nghttp2
    mingw-w64-x86_64-bzip2
)

FAILED=()
for pkg in "${PACKAGES[@]}"; do
    download_and_extract "$pkg" || FAILED+=("$pkg")
done

echo ""
echo "=== Sysroot Summary ==="
echo "Location: $SYSROOT/mingw64"

if [ -d "$SYSROOT/mingw64/lib/pkgconfig" ]; then
    echo "pkg-config files:"
    ls "$SYSROOT/mingw64/lib/pkgconfig/"*.pc 2>/dev/null | wc -l
    echo ""
    echo "Key .pc files present:"
    for pc in gtk4 libadwaita-1 gstreamer-1.0 glib-2.0; do
        if [ -f "$SYSROOT/mingw64/lib/pkgconfig/$pc.pc" ]; then
            echo "  ✓ $pc"
        else
            echo "  ✗ $pc MISSING"
        fi
    done
fi

if [ ${#FAILED[@]} -gt 0 ]; then
    echo ""
    echo "Failed packages:"
    printf '  %s\n' "${FAILED[@]}"
fi
