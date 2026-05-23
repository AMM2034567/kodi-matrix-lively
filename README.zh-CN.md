# Matrix Lively Rust - 一个给lively用的动态壁纸

[English](./README.md)

![预览图](https://github.com/AMM2034567/matrix-lively-rust/blob/master/public/preview.gif)

核心渲染引擎使用 Rust 编写，编译为 WebAssembly，通过 WebGL2 高效运行。

> ⚠ **移植说明**：由于 Lively Wallpaper 仅提供 128 频段的频谱数据，而非原始 PCM 音频，因此无法做到 100% 无损移植。具体表现为波形行为合成数据而非真实 PCM 采样，在 waveform 和 envelope 预设中视觉效果与原版略有差异（波形线更平滑而非颗粒感）。其余功能均已完成对齐。

---

## 项目结构

```
matrix-lively-rust/
├── core/                          # Rust 核心引擎 (WASM)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # WASM 绑定层
│   │   └── renderer.rs            # WebGL2 渲染器
│   └── pkg/                       # wasm-pack 构建输出
├── src/                           # TypeScript 胶水层
│   ├── main.ts                    # Lively 接口与渲染循环
│   └── style.css
├── public/
│   ├── index.html
│   ├── shaders/                   # GLSL 着色器 (与原版相同)
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
│   ├── LivelyInfo.loc.json           # 中文本地化
│   └── LivelyProperties.loc.json     # 中文本地化
├── package.json
├── tsconfig.json
├── README.md
└── README.zh-CN.md
```

## 技术架构

```
Lively Wallpaper
    │
    ├── livelyAudioListener()      ← 128 频段频谱数据
    ├── livelyPropertyListener()   ← 属性变化
    └── livelyCurrentTrack()       ← 专辑封面
    │
    ▼
TypeScript (main.ts)
    │
    ├── props 状态管理
    ├── requestAnimationFrame 渲染循环
    └── MatrixApp (WASM)
        │
        ▼
    Rust / WASM
    ├── update_audio()             音频缓冲区
    ├── update_properties()        属性同步
    ├── update_album_art()         专辑封面
    └── render()                   每帧渲染
            │
            ▼
    MatrixRenderer (WebGL2)
    ├── build_glsl_header()        生成 GLSL 头
    ├── Uniform / 纹理绑定
    └── drawArrays()               绘制全屏四边形
```

## 构建

### 前置要求
- Rust (wasm32-unknown-unknown target)
- Node.js
- wasm-pack

### 命令

```bash
# 开发模式
npm run dev

# 完整构建 (Rust → WASM → TypeScript → Vite)
npm run build

# 构建 + 打包 zip (供 Lively Wallpaper 导入)
npm run pack
```

`npm run pack` 会生成 `matrix-lively-rust.zip`，直接拖入 Lively Wallpaper 即可使用。
