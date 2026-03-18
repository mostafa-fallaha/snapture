# snapture

`snapture` is a native Rust Linux screenshot + annotation MVP built around `egui/eframe` for the editor and `ashpd` for Wayland-friendly screenshot capture through `xdg-desktop-portal`.

## Features

- Capture a screenshot through the screenshot portal
- Open the captured image in an `egui` annotation editor
- Draw freehand pen strokes
- Draw translucent highlight strokes
- Draw rectangles
- Draw arrows
- Place text annotations
- Select, move, and resize existing annotations
- Select and commit a crop
- Extract text from the whole screenshot or the active crop selection
- Undo and redo annotation and crop actions
- Save the final flattened image to PNG
- Copy the final flattened image to the clipboard
- Use `Ctrl+S` to save and `Ctrl+C` to copy the image
- Keep the base screenshot separate from overlays until export

## Project Layout

```text
.
├── Cargo.toml
├── README.md
├── models
│   ├── text-detection.rten
│   └── text-recognition.rten
└── src
    ├── app.rs
    ├── capture
    │   ├── mod.rs
    │   └── portal.rs
    ├── config.rs
    ├── editor
    │   ├── canvas.rs
    │   ├── document.rs
    │   ├── history.rs
    │   └── mod.rs
    ├── error.rs
    ├── main.rs
    ├── model
    │   ├── mod.rs
    │   ├── overlay.rs
    │   └── types.rs
    ├── services
    │   ├── clipboard.rs
    │   ├── ocr.rs
    │   ├── mod.rs
    │   └── save.rs
    ├── tools
    │   ├── arrow.rs
    │   ├── crop.rs
    │   ├── highlighter.rs
    │   ├── mod.rs
    │   ├── pen.rs
    │   ├── rect.rs
    │   └── text.rs
    └── ui
        ├── mod.rs
        ├── toolbar.rs
        └── topbar.rs
```

## Build

```bash
cargo build
```

## Run

```bash
cargo run
```

## Wayland Notes

- Screenshot capture uses `xdg-desktop-portal` through `ashpd`, which is the intended path for GNOME Wayland.
- This MVP intentionally does not try to bypass Wayland with a custom global screen overlay or direct X11-only screen grab APIs.
- On startup, the portal screenshot flow runs before the editor window is created. The `eframe` UI only opens after a screenshot has been returned.
- OCR uses the bundled `models/text-detection.rten` and `models/text-recognition.rten` files from this repository.
- Save uses a `zenity` save dialog on Linux, initialized from the suggested path shown in the left toolbar.
- Clipboard image support depends on the Wayland clipboard stack on the host session.
- Text export uses a system sans font discovered at runtime. If no system font is available, text export will fail until one is installed.

## Arch Linux Packages

Likely packages you will want installed:

```bash
sudo pacman -S --needed base-devel xdg-desktop-portal xdg-desktop-portal-gnome pipewire \
  ttf-dejavu wayland libxkbcommon zenity
```

## Known Limitations

- Portal capture behavior depends on the active desktop portal backend.
- The editor currently centers the image and offers zoom, but not panning.
- Text annotations are add-only in this MVP. Editing an already-placed text overlay is a later improvement.
- Crop keeps overlays as vector objects and filters them by overlay bounds; partial stroke clipping is basic but export-safe.
- There is no global hotkey yet.
