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
    command -v curl > /dev/null && \
        curl --fail --location "$1" --output target/release/neovim-serenade
}


fetch_prebuilt_binary() {
    echo "Downloading binary.."
    url=https://github.com/mattscamp/$name/releases/download/$version/${1}
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
    "Darwin") fetch_prebuilt_binary $name-$version-darwin ;;
    *) echo "No pre-built binary available for ${arch}."; cargo_build ;;
esac

