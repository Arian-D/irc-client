# VeryChat

Simple IRC client meant to implement parts of
[IRCv3](https://modern.ircdocs.horse/).

## Usage
TODO

### NickServ authentication
The Tauri backend sends IRC registration on startup (`NICK`/`USER`) and can identify with NickServ.  
Connection/auth fields can be edited in the app UI (left panel) and applied with the **Connect** button.

### Channel management (libera.chat)
Use the left sidebar to:
- Join a specific channel by entering it (for example, `#rust`) and pressing **Join**
- Leave a specific joined channel with the **Leave** button next to that channel

Messages are sent only to channels you have explicitly joined.

You can still set startup defaults with environment variables:
- `IRC_SERVER` (default: `irc.libera.chat:6667`)
- `IRC_NICK` (default: `uniquenick`)
- `IRC_REAL_NAME` (optional)
- `IRC_NICKSERV_PASSWORD` (optional; enables NickServ auth)
- `IRC_NICKSERV_ACCOUNT` (optional; if omitted, IDENTIFY is sent with password only)

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

## Resources
- https://modern.ircdocs.horse/
- https://datatracker.ietf.org/doc/html/rfc1459
- https://ircv3.net/
- https://www.irchelp.org/
- https://tauri.app/develop/calling-rust/
