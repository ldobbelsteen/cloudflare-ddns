# Cloudflare DDNS

A Rust application for dynamically updating the IP address in Cloudflare records. If you have a dynamic public IP address, this script will make the record always point to you. It uses [icanhazip.com](https://github.com/major/icanhaz) to retrieve your public IP address and the Cloudflare API to edit records.

## Features

- Supports both IPv4 and IPv6 by default (A and AAAA records)
- Cleverly updates records (only when needed)
- Simple console logging
- Built-in timer
- Configurable
- Creating and removing records based on availbable IPs
- Overall reserved and reliable approach to editing records

## Usage

To use the application, it will first need to be compiled. To do so, you will need to have Rust and Cargo installed. Then the easiest way to install is running the following command.

```
cargo install --git https://github.com/ldobbelsteen/cloudflare-ddns
```

It should then be used as follows.

```
cloudflare-ddns <config-file-location>
```

The configuration file is in YAML format and should follow the format in the example config file `example-config.yml` in this repo. Explanations for each of the settings are given there.

## Docker

To help with automation, there is a Dockerfile present to build a lean Docker image. There is a pre-built image available on the GitHub Packages of this repository. An example of how the image can be used:

```
docker run \
    --detach \
    --network host \
    --restart always \
    --volume /path/to/config:/config/cloudflare.yml \
    ghcr.io/ldobbelsteen/cloudflare-ddns:main
```

Setting network access to `host` is needed for IPv6 functionality.
