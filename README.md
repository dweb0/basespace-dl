# 🌋 `basespace-dl`

Download files from projects accross multiple basespace accounts. Regex patterns allow select file download.

[![Build Status](https://travis-ci.com/dweb0/basespace-dl.svg?token=EQz1tk6xqYMBC8vjUmyv&branch=master)](https://travis-ci.com/dweb0/basespace-dl)


## Examples

Download all files from a project

```bash
basespace-dl project17890
```

Download files that match a [regex](https://docs.rs/regex) pattern

```bash
basespace-dl project17890 -p "(A01|B02|F10)"
```

List (do not download) files from project

```bash
basespace-dl project17890 -F
```

## Installation

Via Homebrew or Linuxbrew

```bash
brew install dweb0/basespace-dl
```

From source

```bash
git clone https://github.com/dweb0/basespace-dl
cd basespace-dl
cargo build --release
mv target/release/basespace-dl /usr/local/bin   #optional
```

## Getting started

After installation, you will need to set up your config file. The format is a simple key-value [toml](https://github.com/toml-lang/toml) stored in $HOME/.config/basespace-dl/default.toml. 

```toml
# UserID = "access_token"
11111111 = "youraccesstokenforaccount1goeshere"
22222222 = "youraccesstokenforaccount2goeshere"
33333333 = "youraccesstokenforaccount3goeshere"
```

To link an account, we need to retrieve two things: the access token and its respective userID.

First, retrieve an access token. Go to the [developer dashboard](https://developer.basespace.illumina.com/dashboard). Create a new app. Then, navigate to the "Credentials" tab, and copy the "Access Token" (the client id and secret are not needed).

Once you have your token, run this command and look for the ID field. It should be an integer (e.g. 123480).

```bash
TOKEN="STORE_YOUR_TOKEN_HERE"
curl "https://api.basespace.illumina.com/v1pre3/users/current/" -H "x-access-token: $TOKEN" | python -m json.tool
```

Add the "userID = access_token" pair to the config file. Do this for each account you would like to link.

Go ahead and set the file permissions as readable / writeable by you only.

```bash
chmod 600 $HOME/.config/basespace-dl/default.toml
```

The hardest part is over!

## Acknowledgements

Special thanks to the developers of the all the libraries in Cargo.toml, as well as [Shepmaster](https://stackoverflow.com/users/155423/shepmaster) for help with tokio.
