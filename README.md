# Zulip-sandbag-bot

Watch and report suspicious tournament performance to a zulip topic.

## Usage

Dev settings are provided under `config/base.toml`. You can override these by creating `config/prod.toml`, and/or via environment variables by prefixing the value name with `APP`. Eg: `APP_LICHESS_TOKEN=xxx`