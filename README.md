# VeryChat

Simple IRC client meant to implement [RFC
1459](https://datatracker.ietf.org/doc/html/rfc1459) and parts of
[IRCv3](https://ircv3.net/).

## Usage
TODO

## Building
- Install and configure Tauri based on the [docs](https://v2.tauri.app/start/prerequisites/) for your OS.
- Run `cargo tauri dev`, which will start the webserver (managed by `trunk`) and the system app.
- Code

### Podman container (Optional)
If the simple setup didn't work,
- Ensure `nu` and `podman` are on the path
- Run `source ./scripts/run-in-docker.nu` in nushell
- Code

### Nix (Optional)
If you hate yourself,
- Have a lot of space, patience, and time to waste
- Install nix and enable flakes
- `nix develop .`
- Code
