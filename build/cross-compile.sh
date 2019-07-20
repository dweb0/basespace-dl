#!/usr/bin/env bash

# Must have "cross" installed, only works on linux
# https://github.com/rust-embedded/cross
if [  "$(uname -s)" != "Linux" ]; then
    echo "Must be on Linux to cross compile"
    exit 1
fi

# Have to specify version
if [ "$#" -lt 1 ]
    then
        echo "Usage: ./cross-compile.sh VERSION"
        exit 1
fi
version=$1

# Output everything as zip in target/dist
OUTPUT="target/dist"

rm -rf $OUTPUT
mkdir -p $OUTPUT

# Windows (has .exe in name)
for target in i686-pc-windows-gnu x86_64-pc-windows-gnu; do
    cross build --target $target --release
    rm -rf $OUTPUT/basespace-dl-${version}-${target}
    mkdir $OUTPUT/basespace-dl-${version}-${target}
    cp target/${target}/release/basespace-dl.exe $OUTPUT/basespace-dl-${version}-${target}
    cd $OUTPUT
    zip -r basespace-dl-${version}-${target}.zip basespace-dl-${version}-${target}
    rm -rf basespace-dl-${version}-${target}
    cd ..
done

# Non windows (no .exe in name)
for target in i686-unknown-linux-gnu x86_64-unknown-linux-gnu; do
    cross build --target $target --release
    rm -rf $OUTPUT/basespace-dl-${version}-${target}
    mkdir $OUTPUT/basespace-dl-${version}-${target}
    cp target/${target}/release/basespace-dl $OUTPUT/basespace-dl-${version}-${target}
    cd $OUTPUT
    tar -czvf basespace-dl-${version}-${target}.tar.gz basespace-dl-${version}-${target}
    rm -rf basespace-dl-${version}-${target}
    cd ../..
done
