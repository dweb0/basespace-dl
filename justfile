# https://github.com/casey/just

version := "0.2.1"

# Cross compile for linux and windows, must be run on linux host
cross:
    ./build/cross-compile.sh {{version}}

bundle-mac:
    ./build/bundle-mac.sh {{version}}