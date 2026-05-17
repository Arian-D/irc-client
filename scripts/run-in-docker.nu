#!/usr/bin/env nu

let image_name = "irc-client"

which podman
| if $in == [] {
  alias podman = docker
}

podman build -t $image_name -q -f dev.Dockerfile .

[run build shell]
| input list --fuzzy
| match $in {
  run => "cargo tauri dev",
  build => "cargo tauri build",
  shell => "bash"
}
| (podman run
  --rm
  -it
  -e XDG_RUNTIME_DIR=/tmp
  -e WAYLAND_DISPLAY
  -e IRC_NICK
  -v ($env.XDG_RUNTIME_DIR | path join $env.WAYLAND_DISPLAY):/tmp/($env.WAYLAND_DISPLAY)
  -v (pwd):/app
  irc-client
  bash -c $in)
