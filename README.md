# Java Updater

[![CI](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/mlehmannm/java-updater-rs/actions/workflows/ci.yml)

## Motivation

I have a bunch of computers (mostly Windows) with a couple of different Java versions to maintain. It is cumbersome to download all new versions on all computers, unpack und replace that Java versions.

## Configuration

_TBW_: describe yaml

### Available variables

- `JU_CONFIG_ARCH`
- `JU_CONFIG_TYPE`
- `JU_CONFIG_VENDOR`
- `JU_CONFIG_VERSION`

**NOTE:** The variables used for the directory are not prefixed with `env.`!

## Scheduling (Linux)

_TBW_: cron/systemd

## Scheduling (Windows)

Run as Administrator

```shell
schtasks /create /tn "Update Java Installations" /tr "<path/to/java-update.exe> --config <path/to/config>" /sc onlogon
```

This will run the Java Updater each time the user logs on.

## Notification

### Common variables

All variables will be made available to the executed program via its environment.

- `env.JU_ARCH`
- `env.JU_CONFIG_ARCH`
- `env.JU_CONFIG_TYPE`
- `env.JU_CONFIG_VENDOR`
- `env.JU_CONFIG_VERSION`
- `env.JU_DIRECTORY`
- `env.JU_OS`
- `env.JU_TYPE`
- `env.JU_VENDOR_ID`
- `env.JU_VENDOR_NAME`

### on-failure

Additionally to the common variables, the following variables will be made available as well:

- `env.JU_ERROR`
- `env.JU_OLD_VERSION` (only set, when available)

### on-success (runs always even if no update happened)

Additionally to the common variables, the following variables will be made available as well:

- `env.JU_NEW_VERSION`
- `env.JU_OLD_VERSION` (only set, when available)

### on-update (runs only when an update happened)

Additionally to the common variables, the following variables will be made available as well:

- `env.JU_NEW_VERSION`
- `env.JU_OLD_VERSION` (only set, when available)
