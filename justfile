# https://github.com/casey/just

# Get version from Cargo.toml
version := `egrep "version" Cargo.toml -m 1 | sed -e 's/version *= *//g' -e 's/"//g'`
