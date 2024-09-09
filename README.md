# Cloudflare DDNS

A Rust application for dynamically updating the IP address in Cloudflare
records. If you have a dynamic public IP address, this script will make the
record always point to you. It uses [icanhazip.com](https://icanhazip.com/) to
retrieve your public IP address and the Cloudflare API to edit records.

## Features

- Supports both IPv4 and IPv6 by default (A and AAAA records)
- Cleverly updates records (only when necessary)
- Simple console logging
- Built-in timer
- Configurable
- Creating and removing records based on availbable IPs
- Overall reserved and reliable approach to editing records

## Usage

To use the application, it will first need to be compiled. To do so, you will
need to have Rust and Cargo installed. Then the easiest way to install is
running the following command.

```
cargo install --git https://github.com/ldobbelsteen/cloudflare-ddns
```

It should then be used as follows.

```
cloudflare-ddns <config-file-location>
```

The configuration file is in TOML format and should follow the format in the
example config file `example-config.toml` in this repo. Explanations for each of
the settings are given there.

## Docker

To help with automation, there is a Containerfile present to build a lean
container image. There is a pre-built image available on the GitHub Packages of
this repository. An example of how the image can be used:

```
docker run \
    --detach \
    --restart on-failure \
    --volume /path/to/config:/config.toml \
    ghcr.io/ldobbelsteen/cloudflare-ddns
```
