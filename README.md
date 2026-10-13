# Craft Suite

The Craft apps in one repository, with ArtCraft running on your own cloud accounts (Vertex AI,
Bedrock, Jev routing) and the apps handing files to each other.

Each app is a folder brought in with `git subtree`, so it keeps its own workspace, build and
docs, and can be updated from its upstream.

| Folder           | App            | Binary              | What it is                                        |
|------------------|----------------|---------------------|---------------------------------------------------|
| `artcraft`       | ArtCraft       | `artcraft-native`   | AI image, video and music generation (local mode) |
| `filmcraft`      | FilmCraft      | `filmcraft`         | video editor                                      |
| `effectcraft`    | EffectCraft    | `effectcraft`       | compositing and motion graphics                   |
| `photocraft`     | PhotoCraft     | `photocraft`        | layered image editor                              |
| `lightcraft`     | LightCraft     | `lightcraft`        | photo catalog and grading                         |
| `vectorcraft`    | VectorCraft    | `vectorcraft`       | vector illustration                               |
| `designcraft`    | DesignCraft    | `designcraft`       | page layout                                       |
| `deckcraft`      | DeckCraft      | `deckcraft`         | presentations                                     |
| `gridcraft`      | GridCraft      | `gridcraft`         | spreadsheets                                      |
| `cadcraft`       | CADCraft       | `cadcraft`          | CAD and drafting                                  |
| `printcraft`     | PrintCraft     | `pdfcraft`          | PDF editor                                        |
| `craft-launcher` | Craft Launcher | `artcraft-launcher` | launcher for the apps                             |

## Building

Build each app in its folder (release builds are what ArtCraft looks for first):

```sh
cd artcraft    && cargo build --release -p artcraft_native
cd filmcraft   && cargo build --release -p filmcraft
cd photocraft  && cargo build --release -p photocraft
cd printcraft  && cargo build --release -p pdfcraft
# …and likewise effectcraft, lightcraft, vectorcraft, designcraft, deckcraft, gridcraft, cadcraft
```

Each app's own README lists its system requirements (FFmpeg for FilmCraft and EffectCraft,
cmake for ArtCraft, and so on). ArtCraft's local-mode setup is in
[`artcraft/artcraft-native/README.md`](artcraft/artcraft-native/README.md).

## Hand-offs

**ArtCraft → apps.** In ArtCraft's lightbox, the **Craft apps** row sends the item to:

| App         | Takes               | What happens                               |
|-------------|---------------------|--------------------------------------------|
| FilmCraft   | image, video, audio | imported into the open project             |
| EffectCraft | image, video, audio | imported into the open project             |
| PhotoCraft  | image               | opened as a document                       |
| LightCraft  | image               | added to the catalog                       |
| VectorCraft | image               | placed in the open document                |
| DesignCraft | image               | placed in the open layout                  |
| DeckCraft   | image               | inserted on the current slide              |
| GridCraft   | image               | inserted as a picture in the open workbook |
| PrintCraft  | image               | made into a PDF                            |

ArtCraft starts the app if it isn't running (finding the build in this checkout by its own
location, or `CRAFT_SUITE_DIR`, `/Applications`, or `PATH`) and talks to it over a loopback
control port after that. CADCraft has no button yet: it can't attach images (IMAGEATTACH
is on its upstream gaps list).

**Apps → ArtCraft.** FilmCraft (File › Send to ArtCraft › Current Frame / Active Sequence) and
PhotoCraft (File › Export › Send to ArtCraft) write into `~/Artcraft/inbox/`. ArtCraft picks new
files up within a couple of seconds and adds them to its library. Any file you drop in that
folder is imported the same way.

## Updating from upstream

```sh
scripts/sync-upstream.sh             # every app
scripts/sync-upstream.sh filmcraft   # one app
```

Local changes to an app (such as the Send to ArtCraft commands) are kept by the subtree merge;
conflicts are resolved like any merge.
