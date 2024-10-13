# Java Updater

[![CI](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml)

## Motivation

I have a bunch of computers (mostly Windows) with a couple of different Java versions to maintain. It is cumbersome to download all new versions on all computers, unpack und replace that Java versions.

## Configuration

_TBW_: describe yaml

## Scheduling (Linux)

_TBW_: cron/systemd

## Scheduling (Windows)

Run as Adminstrator

```shell
schtasks /create /tn "Update Java Installations" /tr "<path/to/java-update.exe> --config <path/to/config>" /sc onlogon
```

This will run the Java Updater each time the user logs on.

## Notification

### Common variables

- JU_ARCH
- JU_INSTALLATION
- JU_TYPE
- JU_VENDOR_ID
- JU_VENDOR_NAME

### on-update

- JU_NEW_VERSION
- JU_OLD_VERSION (only set, when available)

### on-failure

- JU_ERROR
- JU_OLD_VERSION (only set, when available)
