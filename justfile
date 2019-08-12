# https://github.com/casey/just

# Get version from Cargo.toml
version := `egrep "version" Cargo.toml -m 1 | sed -e 's/version *= *//g' -e 's/"//g'`

publish-tag:
    git tag -a {{version}} && git push origin {{version}}

publish-crate:
    # cargo package -> inspect crate
    # cargo publish