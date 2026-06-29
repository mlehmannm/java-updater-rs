# Java Updater

[![CI](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml)

## Motivation

I have a bunch of computers (mostly Windows) with a couple of different Java versions to maintain. It is cumbersome to download all new versions on all computers, unpack und replace that Java versions.

## Configuration

_TBW_: describe yaml

### Available variables

Java Updater variables can be referenced as `${NAME}`.

- `JU_CONFIG_ARCH`
- `JU_CONFIG_DIRECTORY`
- `JU_CONFIG_TYPE`
- `JU_CONFIG_VENDOR`
- `JU_CONFIG_VERSION`
- `JU_ARCH`
- `JU_FAMILY`
- `JU_OS`

Operating system environment variables can be referenced as `${env.NAME}`.

## Scheduling (Linux)

_TBW_: cron/systemd

## Scheduling (Windows)

Run as Administrator

```shell
schtasks /create /tn "Update Java Installations" /tr "<path/to/java-updater.exe> --config <path/to/config>" /sc onlogon
```

This will run the Java Updater each time the user logs on.

## Notification

### Common variables

Java Updater variables can be referenced as `${NAME}` in notify command configuration. Operating system environment variables can be referenced as `${env.NAME}`.

All common variables will also be made available to the executed program via its environment.

- `JU_ARCH`
- `JU_CONFIG_ARCH`
- `JU_CONFIG_DIRECTORY`
- `JU_CONFIG_TYPE`
- `JU_CONFIG_VENDOR`
- `JU_CONFIG_VERSION`
- `JU_DIRECTORY`
- `JU_FAMILY`
- `JU_OS`
- `JU_TYPE`
- `JU_VENDOR_ID`
- `JU_VENDOR_NAME`

### on-failure

Additionally to the common variables, the following variables will be made available as well:

- `JU_ERROR`
- `JU_OLD_VERSION` (only set, when available)

### on-success (runs always even if no update happened)

Additionally to the common variables, the following variables will be made available as well:

- `JU_NEW_VERSION`
- `JU_OLD_VERSION` (only set, when available)

### on-update (runs only when an update happened)

Additionally to the common variables, the following variables will be made available as well:

- `JU_NEW_VERSION`
- `JU_OLD_VERSION` (only set, when available)
