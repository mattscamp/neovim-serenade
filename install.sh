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
    if [ "$2" = ".tar.gz" ]; then
        command -v curl > /dev/null && curl -L $1 --output target/release/${name} && tar -xzf target/release/${name}
    else
    	command -v curl > /dev/null && curl -L $1 --output target/release/${name}${2}
    fi	
}

fetch_prebuilt_binary() {
    echo "Downloading binary.."
    url=https://github.com/mattscamp/${name}/releases/download/${version}/${name}-${1}${2}
    echo $url
    mkdir -p target/release

    if (download "${url}" ${2}); then
        chmod a+x target/release/${name}
        return
    else
        cargo_build || echo "Prebuilt binaries are not ready for this platform."
    fi
}

arch=$(uname)
case "${arch}" in
    "Darwin") fetch_prebuilt_binary "-macos-x86_64" ".dmg" ;;
    "Linux") fetch_prebuilt_binary "-linux-x86_64" ".tar.gz" ;;
    "WindowsNT") fetch_prebuilt_binary "-windows-x86_64" ".exe" ;;
    *) echo "No pre-built binary available for ${arch}."; cargo_build ;;
esac

