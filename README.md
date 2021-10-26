# Cloudflare DDNS

A Python script for dynamically updating the IP address in Cloudflare records. If you have a dynamic public IP address, this script will make the record always point to you. It uses [icanhazip.com](https://github.com/major/icanhaz) to retrieve your public IP address and the Cloudflare API to edit records.

## Features

- Supports both IPv4 and IPv6 by default (A and AAAA records)
- Cleverly updates records (only when needed)
- Simple console logging
- Built-in timer
- Configurable
- Morphing (create and remove records based on availbable IPs)
- Overall reserved and reliable approach to editing records

## Usage

This script has dependencies which can be installed with `pip install -r requirements.txt`. Then running it is simply done as follows.

```
python cloudflare.py <path-to-config-file>
```

The configuration file is in YAML format and should follow the format in the example config file in this repo `example-config.yml`. Explanations for each of the settings is given there.

## Automation

There are several ways of automatically running this script to keep records up-to-date in the background. Below are examples of some of these.

### Docker

There is a Dockerfile present to build a lean Docker image of this script. There is a pre-built image available on the GitHub Packages of this repository. An example of how the image can be used:

```
docker run \
    --detach \
    --network host \
    --restart always \
    --volume /path/to/config:/config/cloudflare.yml \
    ghcr.io/ldobbelsteen/cloudflare-ddns:main
```

Setting network access to `host` is needed for IPv6 functionality.

### Systemd

Running the script in systemd is as simple as running any Python script in systemd. Create the file `/etc/systemd/system/cloudflare-ddns.service`

```
[Unit]
Description=Cloudflare dynamic DNS script

[Service]
Type=simple
ExecStart=/usr/bin/python -u /path/to/cloudflare.py /path/to/config.yml

[Install]
WantedBy=multi-user.target
```

Make sure to enable the timer in the config when using systemd. To enable the service, run the following commands:

```
systemctl daemon-reload
systemctl start cloudflare-ddns
systemctl enable cloudflare-ddns
```
