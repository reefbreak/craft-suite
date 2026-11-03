# ArtCraft brand marks: removed from this build

This copy of DesignCraft is a modified build in the Gesso craft-suite. It is not an official
DesignCraft or ArtCraft release.

[`LICENSE-brand.txt`](LICENSE-brand.txt) requires anyone who distributes a modified version of
DesignCraft to remove the ArtCraft Marks (the ArtCraft name used as a brand, the wordmark and the
logos) and not to present it as an ArtCraft product. So the wordmark and mark files that upstream
keeps in this folder (`artcraft-logo*.png/.svg` and `artcraft-mark*.png/.svg`) have been deleted.
The app no longer draws the mark either (it used to paint its outline from path data copied out of
`artcraft-mark.svg`, in ArtCraft brand blue): the About splash now starts with the name
"DesignCraft" and its version, and the start screen community card shows its title, text and Discord
button without the mark. The README carries no logo images.

The ArtCraft name still appears in plain text where the licence allows it: to say that this build is
based on DesignCraft by the ArtCraft team, and to label links to the upstream project (its Discord
and getartcraft.com). `LICENSE-brand.txt` is kept unchanged for reference.

Do not add the mark files back to this build.
