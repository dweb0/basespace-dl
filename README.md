# basespace-dl

[![Build Status](https://travis-ci.org/dweb0/basespace-dl.svg?branch=master)](https://travis-ci.org/dweb0/basespace-dl)
[![Build status](https://ci.appveyor.com/api/projects/status/c0yglngi1lgc2sox?svg=true)](https://ci.appveyor.com/project/dweb0/basespace-dl)
[![GitHub release](https://img.shields.io/github/release/dweb0/basespace-dl)](https://github.com/dweb0/basespace-dl/releases)

Download files from projects across multiple basespace accounts.

## Features
* Easy syntax `basespace-dl PROJECT`
* Multiple accounts (one time config file setup)
* Fast (concurrent fetching and downloading)
* Only download what you need (using regex patterns)

## Demo

![Demo](screencast.svg)

> Demo data from Basespace's "Public Data"

## Examples

List ALL projects

```bash
basespace-dl ALL
```

Download all files from a project

```bash
basespace-dl project17890
```

Download files that match a [regex](https://docs.rs/regex) pattern

```bash
basespace-dl project17890 -p "(A01|B02|F10)"
```

Include Undetermined files

```bash
basespace-dl project17890 -U
```

List files from a specific project

```bash
basespace-dl project17890 -F
```

For some more advanced examples of what you can do, check out the [cookbook](cookbook.md).

## Installation

Using macOS Homebrew or Linuxbrew

```
brew install dweb0/all/basespace-dl
```

To upgrade to the latest version

```
brew update
brew upgrade basespace-dl
```

Using cargo (for rust developers)

```
cargo install --git https://github.com/dweb0/basespace-dl
```

Or you can download a pre-built binary for Mac, Windows, or Linux on the [releases page](https://github.com/dweb0/basespace-dl/releases).

## Getting started

After installation, you will need to set up your config file. The format is a simple [key-value toml](https://github.com/toml-lang/toml#user-content-keyvalue-pair) stored in ~/.config/basespace-dl/default.toml. 

```toml
# UserID = "access_token"
11111111 = "youraccesstokenforaccount1goeshere"
22222222 = "youraccesstokenforaccount2goeshere"
33333333 = "youraccesstokenforaccount3goeshere"
```

To link an account, we need to retrieve two things: the access token and its respective userID.

### Getting access token

1. Go to the [developer dashboard](https://developer.basespace.illumina.com/dashboard). 
2. Create a new app. 
3. Navigate to the "Credentials" tab, and copy the "Access Token".

### Getting user ID

Now that you have your token, we can run a curl command to get your user ID. Look for the "Id" field
in the output.

```bash
TOKEN="STORE_YOUR_TOKEN_HERE"
curl "https://api.basespace.illumina.com/v1pre3/users/current/" -H "x-access-token: $TOKEN"
```

You can also go to https://api.basespace.illumina.com/v1pre3/users/current while logged in to see the same thing.

### Final steps

Add the "user_id = access_token" pair to the config file. Do this for each account you would like to link.

Note: It's a good idea to set the file permissions as readable / writeable by only you.

```bash
chmod 600 ~/.config/basespace-dl/default.toml
```

Now you're ready to go! 

Try running a command to see if it works (assuming you have projects in your account).

```bash
basespace-dl ALL
```
