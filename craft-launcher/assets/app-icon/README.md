# Gesso Launcher app icon

The app icon is **Gesso's app icon** (a dark canvas with one brush stroke of white ground and a
gold dot), copied unchanged from Gesso's
[`gesso/assets/brand/gesso.svg`](../../../gesso/gesso/assets/brand/gesso.svg) into
`gesso-launcher.svg`. Every image in this directory is rendered from it, or from
`gesso-mark-black.svg`, by `packaging/icons.sh`.

These files are original artwork for Gesso, licensed MIT OR Apache-2.0 like the rest of Gesso
([`LICENSE.txt`](LICENSE.txt)). The upstream ArtCraft Launcher's icon, the ArtCraft "A" mark, is
an ArtCraft trademark and has been removed from this build
([`docs/brand/README.md`](../../docs/brand/README.md)).

## Files

- `gesso-launcher.svg`: the source; also the hicolor scalable icon.
- `gesso-mark-black.svg`: the brush stroke and dot alone (from Gesso's `gesso-mark.svg`), in
  black on transparent, on a square view box: the source of the macOS menu-bar template.
- `gesso-launcher-1024.png`: 1024 px render, with extra transparent margin for Apple's icon grid.
- `gesso-launcher.icns`: macOS bundle icon (`CFBundleIconFile`).
- `gesso-launcher.ico`: Windows icon, 16–256 px, embedded in the `.exe` by
  `apps/artcraft-launcher/build.rs` and used by the MSI.
- `hicolor/<size>/apps/us.reefbreak.gesso-launcher.png` (16–512) and
  `hicolor/scalable/apps/us.reefbreak.gesso-launcher.svg`: Linux icon theme.
- `tray-template-44.png`: the macOS menu-bar template (22 pt @2x; macOS tints it).
- `tray-64.png`: the tray icon on Windows, Linux and FreeBSD (the app icon).

## Regenerate

Run `packaging/icons.sh` on any OS. It needs `resvg`; the `.ico` and `.icns` are packed by
`cargo xtask ico` / `cargo xtask icns`. Never edit the renders by hand: change the SVGs (and, if
Gesso's icon changes, copy the new `gesso.svg` over `gesso-launcher.svg`).
