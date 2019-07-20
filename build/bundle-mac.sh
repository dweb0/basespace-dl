#!/usr/bin/env bash

# Have to specify version
if [ "$#" -lt 1 ]
    then
        echo "Usage: ./bundle-mac.sh VERSION"
        exit 1
fi

version=$1

# Output everything as zip in target/dist
OUTPUT="target/dist"
target="x86_64-apple-darwin"

rm -rf $OUTPUT
mkdir -p $OUTPUT

mkdir $OUTPUT/basespace-dl-${version}-${target}
cp target/release/basespace-dl $OUTPUT/basespace-dl-${version}-${target}
cd $OUTPUT
tar -czvf basespace-dl-${version}-${target}.tar.gz basespace-dl-${version}-${target}
rm -rf basespace-dl-${version}-${target}
