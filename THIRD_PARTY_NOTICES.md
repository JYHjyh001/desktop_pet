# PetDrawer Third-Party Notices

This file covers third-party material that is distributed with PetDrawer. It
does not grant a license to PetDrawer's own source code or branding. The
project's own license remains to be selected by the project owner.

## Twemoji graphics

PetDrawer includes a selected set of Twemoji SVG graphics for the local chat
emoji picker and message display.

- Project: Twemoji
- Copyright: Twitter, Inc. and other contributors
- Source: https://github.com/twitter/twemoji
- Graphics license: Creative Commons Attribution 4.0 International
- License: https://creativecommons.org/licenses/by/4.0/
- PetDrawer changes: selected glyphs are packaged and integrated into the
  application UI; no intentional artwork modifications are documented in this
  repository.

Twemoji's software and graphics use separate licenses. PetDrawer redistributes
the SVG graphics under CC BY 4.0.

## Runtime libraries

PetDrawer also uses Vue, Three.js, Tauri and their dependency trees. Direct
dependency versions and declared SPDX licenses are recorded in
`docs/third-party-license-and-assets-audit.md`. A complete release still needs
an automatically generated transitive dependency inventory and review of all
license texts and upstream notices.
