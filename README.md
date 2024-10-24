# Souris

[![wakatime](https://wakatime.com/badge/github/BurntNail/Souris.svg)](https://wakatime.com/badge/github/BurntNail/Souris)

A tiny `no_std` database designed to minimise size over all else. If you're using `sourisd`, then it's also all stored in-memory for ultimate speed!

Named after mice, because they're tiny.

## Usage
Use the `sourisd` service to run a daemon on your local machine, and then `souris` to modify local databases or to modify `sourisd` databases.

## NB:
This project is currently not far off being finished but also not that close. I also have a major problem with endless scope creep (which in fairness, isn't really a problem if I'm learning new things).

## Running of `sourisd`

You can either make a directory for the data (eg. `mkdir data && chmod 777 data`) and docker compose like this example:
```yaml
services:
  watchtower:
    image: containrrr/watchtower
    command:
      - "--label-enable"
      - "--interval"
      - "30"
      - "--rolling-restart"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock

  souris:
    image: ghcr.io/burntnail/sourisd:latest
    restart: always
    environment:
      - RUST_LOG=info
      - BASE_LOCATION=/sourisdata/
    volumes:
      - ./data:/sourisdata/
    labels:
      - "com.centurylinklabs.watchtower.enable=true"
    ports:
      - "7687:7687"
    expose:
      - 7687
```

This one conveniently includes watchtower, and so will auto-restart and update whenever the docker image is updated.

Or build it locally and use something like this systemd file:
```
[Unit]
Description=SourisDB Daemon
After=multi-user.target

[Service]
ExecStart=/usr/local/bin/sourisd
Type=simple
Restart=on-failure
Environment="RUST_LOG=trace"

[Install]
WantedBy=default.target
```
To use the systemd file, I put this into `/etc/systemd/system/sourisd.service`, then ran a quick `sudo systemctl daemon-reload && sudo systemctl enable --now sourisd`.