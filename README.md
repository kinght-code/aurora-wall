# aurora-wall

`aurora-wall` is a Linux wallpaper manager with live wallpaper support and matching boot-theme export tools.

It manages local still and video wallpapers, applies them quietly in the background, and can generate matching GRUB and Plymouth themes from the same media. Hyprland is the most integrated path today, and the app also detects generic Wayland, X11, and desktop fallback sessions at runtime. The longer implementation roadmap is in [plan.txt](/home/ahnaf/Documents/wallpaper/plan.txt).

## Quick start

Install runtime dependencies:

```bash
sudo pacman -S mpvpaper mpv
sudo pacman -S swww
```

If you prefer `awww` for still wallpapers instead of `swww`:

```bash
sudo pacman -S awww
```

If you want Plymouth boot themes:

```bash
sudo pacman -S plymouth
```

If your dotfiles use `pywal` and you want application theming to follow the wallpaper:

```bash
sudo pacman -S pywal
```

Build the app:

```bash
cd aurora-wall
cargo build --release
```

## Wallpaper workflow

Detect your session and outputs:

```bash
cargo run -q -p aurora-wall-cli -- doctor
cargo run -q -p aurora-wall-cli -- list-outputs
```

`doctor` prints the detected backend, session variables, config path, state path, and available wallpaper tools.

Create a config and assign wallpapers:

```bash
cargo run -q -p aurora-wall-cli -- init-config
cargo run -q -p aurora-wall-cli -- set-image --output eDP-1 --path /path/to/still.jpg
cargo run -q -p aurora-wall-cli -- set-video --output eDP-1 --path /path/to/live.mp4
```

Apply the saved setup:

```bash
cargo run -q -p aurora-wall-cli -- apply
```

`apply` returns immediately and leaves live wallpaper playback running in the background. If `wal` is installed, it also regenerates your `pywal` theme automatically from the active still wallpaper or from a poster frame of the active video wallpaper.

Use verbose mode only when you want command details or child-process logs:

```bash
cargo run -q -p aurora-wall-cli -- apply -v
```

Skip `pywal` syncing when needed:

```bash
cargo run -q -p aurora-wall-cli -- apply --no-wal
```

Replay the last successfully applied wallpaper snapshot instead of the current config:

```bash
cargo run -q -p aurora-wall-cli -- apply --restore
```

Useful config commands:

```bash
cargo run -q -p aurora-wall-cli -- show-config
cargo run -q -p aurora-wall-cli -- remove-output --output HDMI-A-1
```

## Boot theme workflow

GRUB cannot play live video. The correct boot pipeline is:

- extract a poster frame from the wallpaper video
- use that poster for GRUB
- use the same poster or source video to generate a matching Plymouth theme

One-command export:

```bash
cargo run -q -p aurora-wall-cli -- export-boot-theme \
  --video /path/to/live.mp4 \
  --poster /tmp/aurora-wall-poster.png \
  --grub-dir /tmp/aurora-wall-grub \
  --plymouth-dir /tmp/aurora-wall-plymouth \
  --title "Aurora Wall" \
  --at 00:00:03
```

Install the generated boot themes:

```bash
sudo cargo run -q -p aurora-wall-cli -- install-boot-theme \
  --grub-dir /tmp/aurora-wall-grub \
  --plymouth-dir /tmp/aurora-wall-plymouth
```

If you want manual control, you can still use:

- `export-video-poster`
- `export-grub-theme`
- `export-plymouth-theme`

## Command reference

### Summary

```bash
aurora-wall doctor
aurora-wall init-config
aurora-wall list-outputs
aurora-wall set-image --output eDP-1 --path /path/to/still.jpg
aurora-wall set-video --output eDP-1 --path /path/to/live.mp4
aurora-wall apply
aurora-wall export-boot-theme --video /path/to/live.mp4 --title "Aurora Wall"
sudo aurora-wall install-boot-theme
```

### Help

```text
aurora-wall
  doctor
  init-config [--config PATH]
  show-config [--config PATH]
  list-outputs
  remove-output --output NAME [--config PATH]
  set-image --output NAME --path FILE [--scaling fill|fit|center] [--transition none|fade] [--config PATH]
  set-video --output NAME --path FILE [--loop infinite|once] [--mute yes|no] [--config PATH]
  apply [--config PATH] [--restore] [--no-wal] [-v|--verbose]
  export-video-poster --video FILE [--output FILE] [--at 00:00:03]
  export-boot-theme --video FILE [--poster FILE] [--grub-dir PATH] [--plymouth-dir PATH] [--title TEXT] [--at 00:00:03]
  install-boot-theme [--grub-dir PATH] [--plymouth-dir PATH]
  export-grub-theme [--config PATH] [--theme-dir PATH] [--background FILE] [--title TEXT]
  export-plymouth-theme [--config PATH] [--theme-dir PATH] [--background FILE] [--video FILE] [--title TEXT] [--at 00:00:03]
  status [--config PATH]
  print-service
  print-pkgbuild
```

### Commands

- `doctor`: detect the active backend, print session readiness details, and show config/state paths plus available tools.
- `init-config`: create a default config file in the standard config location or a custom path.
- `show-config`: print the currently loaded wallpaper configuration.
- `list-outputs`: list detected monitor/output names from the current session.
- `remove-output`: remove saved wallpaper entries for an output that no longer exists.
- `set-image`: assign a still wallpaper to an output and save it into config.
- `set-video`: assign a live wallpaper video to an output and save it into config.
- `apply`: apply the saved wallpaper configuration; runs quietly by default, syncs `pywal` automatically when available, supports `-v` or `--no-wal`, and can replay the last applied snapshot with `--restore`.
- `export-video-poster`: extract a still frame from a video for boot-theme use.
- `export-boot-theme`: generate poster, GRUB theme, and Plymouth theme from one video input.
- `install-boot-theme`: install the generated GRUB and Plymouth themes into system boot locations.
- `export-grub-theme`: generate a GRUB theme from a still image.
- `export-plymouth-theme`: generate a Plymouth boot splash theme from a still image or a video source.
- `status`: show current app status, config path, state path, and last applied state.
- `print-service`: print the systemd user service definition for wallpaper restore.
- `print-pkgbuild`: print the PKGBUILD template for packaging on Arch Linux.

## Notes

- Still wallpapers use `swww` or `awww`.
- Live wallpapers use `mpvpaper` and `mpv`.
- Backend selection is runtime-based: Hyprland is preferred when available, then generic Wayland, X11, and desktop fallback.
- If `pywal` is installed, `apply` syncs application theming from the active wallpaper automatically.
- `apply` is quiet by default and starts live wallpaper playback detached from the terminal.
- `apply --restore` uses the last successfully applied wallpaper snapshot saved in the state file.
- Boot theme generation works without root, but installation into GRUB and Plymouth locations requires `sudo`.

## Project files

- Example config: [aurora-wall/examples/config.conf](/home/ahnaf/Documents/wallpaper/aurora-wall/examples/config.conf)
- Arch packaging: [aurora-wall/packaging/PKGBUILD](/home/ahnaf/Documents/wallpaper/aurora-wall/packaging/PKGBUILD)
- systemd user service: [aurora-wall/packaging/aurora-wall.service](/home/ahnaf/Documents/wallpaper/aurora-wall/packaging/aurora-wall.service)
