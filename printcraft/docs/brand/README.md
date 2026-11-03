# ArtCraft brand marks: removed from this build

This copy of PdfCraft is a modified build in the Gesso craft-suite. It is not an official PdfCraft
or ArtCraft release.

[`LICENSE-brand.txt`](LICENSE-brand.txt) requires anyone who distributes a modified version of
PdfCraft to remove the ArtCraft Marks (the ArtCraft name used as a brand, the wordmark and the
logos) and not to present it as an ArtCraft product. So the wordmark and mark files that upstream
keeps in this folder (`artcraft-logo*.png/.svg` and `artcraft-mark*.png/.svg`, and their
`ATTRIBUTION.toml` entries) have been deleted. The app no longer draws them either: the Home screen
community card shows its title and text without the mark, and the About dialog shows the name and
version in plain text, with a plain-text line saying the build is based on PdfCraft by the ArtCraft
team where the "Part of" wordmark used to be. The home-screen screenshot
`docs/images/pdfcraft-tools.png` had the mark painted out. The README carries no logo images.

The ArtCraft name still appears in plain text where the licence allows it: to say that this build is
based on PdfCraft by the ArtCraft team, and to label links to the upstream project (its Discord and
getartcraft.com). `LICENSE-brand.txt` is kept unchanged for reference.

Do not add the mark files back to this build.
