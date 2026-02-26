#!/usr/bin/env nu

let image_name = "irc-client"

which podman
| if $in == [] {
  alias podman = docker
}

podman build -t $image_name -f dev.Dockerfile .

(podman run
  --rm
  -it
  --entrypoint bash
  -e XDG_RUNTIME_DIR=/tmp
  -e WAYLAND_DISPLAY
  -v ($env.XDG_RUNTIME_DIR | path join $env.WAYLAND_DISPLAY):/tmp/($env.WAYLAND_DISPLAY)
  -v (pwd):/app
  irc-client)
