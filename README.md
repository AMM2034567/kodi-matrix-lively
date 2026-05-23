# Matrix Lively Rust - A dynamic wallpaper for Lively

[简体中文](./README.zh-CN.md)

![Preview](https://github.com/AMM2034567/matrix-lively-rust/blob/master/public/preview.gif)

A Lively Wallpaper port of the Kodi Matrix visualization addon.
The core renderer is written in Rust, compiled to WebAssembly, and runs on WebGL2.

> ⚠ **Porting note**: Lively Wallpaper only provides 128-band frequency data instead of raw PCM audio, so 100% lossless porting is not possible. The waveform row contains synthesized data rather than real PCM samples, making the waveform and envelope presets slightly smoother than the original. All other features are fully aligned.

---

## Project Structure

```
matrix-lively-rust/
├── core/                          # Rust engine (WASM)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # WASM bindings
│   │   └── renderer.rs            # WebGL2 renderer
│   └── pkg/                       # wasm-pack output
├── src/                           # TypeScript glue
│   ├── main.ts                    # Lively interface & render loop
│   └── style.css
├── public/
│   ├── index.html
│   ├── shaders/                   # GLSL shaders (same as original)
│   │   ├── logo.frag.glsl
│   │   ├── album.frag.glsl
│   │   ├── nologo.frag.glsl
│   │   ├── nologowf.frag.glsl
│   │   ├── nologowfenv.frag.glsl
│   │   ├── clean.frag.glsl
│   │   ├── cleanwf.frag.glsl
│   │   └── cleanwfenv.frag.glsl
│   ├── textures/
│   │   ├── logo.png
│   │   └── noise.png
│   ├── LivelyInfo.json
│   ├── LivelyProperties.json
│   ├── LivelyInfo.loc.json
│   └── LivelyProperties.loc.json
├── package.json
├── tsconfig.json
├── README.md
└── README.zh-CN.md
```

## Architecture

```
Lively Wallpaper
    │
    ├── livelyAudioListener()      ← 128-bin frequency data
    ├── livelyPropertyListener()   ← property changes
    └── livelyCurrentTrack()       ← album art
    │
    ▼
TypeScript (main.ts)
    │
    ├── props state management
    ├── requestAnimationFrame loop
    └── MatrixApp (WASM)
        │
        ▼
    Rust / WASM
    ├── update_audio()             fill audio buffer
    ├── update_properties()        sync properties
    ├── update_album_art()         upload album art
    └── render()                   per-frame draw
            │
            ▼
    MatrixRenderer (WebGL2)
    ├── build_glsl_header()        generate GLSL header
    ├── uniforms / texture binding
    └── drawArrays()               fullscreen quad
```

## Build

### Prerequisites
- Rust (wasm32-unknown-unknown target)
- Node.js
- wasm-pack

### Commands

```bash
# Dev server with hot reload
npm run dev

# Full build (Rust → WASM → TypeScript → Vite)
npm run build

# Build + pack zip (ready for Lively Wallpaper)
npm run pack
```

`npm run pack` generates `matrix-lively-rust.zip` — drag & drop into Lively Wallpaper.
