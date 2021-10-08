#!/usr/bin/env sh

set -o errexit

version=v0.1.0
name=neovim-serenade

cargo_build() {
    if command -v cargo > /dev/null; then
        echo "Trying to build locally using Cargo.."
        cargo build --release
    else
        echo "Could not build binary. Your installation might be corrupt."
        return 1
    fi
}

download() {
    command -v curl > /dev/null && curl -L $1 | tar -xz -C target/release/
}


fetch_prebuilt_binary() {
    echo "Downloading binary.."
    url=https://github.com/mattscamp/$name/releases/download/$version/$name-${1}
    echo $url
    mkdir -p target/release

    if (download "$url"); then
        chmod a+x target/release/neovim-serenade
        return
    else
        cargo_build || echo "Prebuilt binaries are not ready for this platform."
    fi
}

arch=$(uname)
case "${arch}" in
    "Darwin") fetch_prebuilt_binary x86_64-apple-darwin.tar.gz ;;
    "Linux") fetch_prebuilt_binary x86_64-unknown-linux-gnu.tar.gz ;;
    #"WindowsNT") fetch_prebuilt_binary x86_64-pc-windows-msvc.zip ;;
    *) echo "No pre-built binary available for ${arch}."; cargo_build ;;
esac

