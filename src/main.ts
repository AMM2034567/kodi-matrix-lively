import init, { MatrixApp } from '../core/pkg/matrix_wasm_core';

let app: MatrixApp | null = null;
let currentAudioData: Float32Array = new Float32Array(128);

let props = {
    red: 51,
    green: 204,
    blue: 255,
    rainHighlights: 70,
    intensity: 1.5,
    distortion: 60,
    dotSize: 3,
    preset: 0,
    crtCurve: false,
    fallSpeed: 25,
    noiseFluctuation: 25,
    dotMode: false,
    lowPower: false
};

const presets = [
    "logo.frag.glsl",
    "album.frag.glsl",
    "nologo.frag.glsl",
    "nologowf.frag.glsl",
    "nologowfenv.frag.glsl",
    "clean.frag.glsl",
    "cleanwf.frag.glsl",
    "cleanwfenv.frag.glsl"
];

let logoData: any = null;
let noiseData: any = null;
let currentAlbumData: { data: Uint8Array, width: number, height: number } | null = null;
let canvas: HTMLCanvasElement | null = null;

(window as any).livelyAudioListener = function(audioArray: number[]) {
    for (let i = 0; i < 128; i++) {
        currentAudioData[i] = audioArray[i];
    }
};

(window as any).livelyPropertyListener = function(name: string, val: any) {
    if (name in props) {
        (props as any)[name] = val;
        
        if (name === 'preset' || name === 'lowPower') {
            // Preset or lowPower changed, reload app with new shader
            loadApp();
        } else if (app) {
            // Update properties instantly
            app.update_properties(props.red, props.green, props.blue, props.rainHighlights, props.intensity, props.distortion, props.dotSize, props.crtCurve ? 1.0 : 0.0, props.fallSpeed, props.noiseFluctuation, props.dotMode ? 1.0 : 0.0, props.lowPower ? 1.0 : 0.0);
        }
    }
};

let albumLoadVersion = 0;

(window as any).livelyCurrentTrack = function(data: string) {
    console.log("[Album] livelyCurrentTrack called, data length:", data?.length ?? 0);
    try {
        const obj = JSON.parse(data);
        if (obj == null) {
            console.log("[Album] data is null (no media playing)");
            return;
        }
        console.log("[Album] Track:", obj.Title ?? "unknown", "| Artist:", obj.Artist ?? "unknown", "| Has thumbnail:", obj.Thumbnail != null);
        if (obj.Thumbnail == null) return;

        const myVersion = ++albumLoadVersion;
        const base64 = obj.Thumbnail as string;

        // Try PNG first, then JPEG — Lively may send either format
        const tryLoad = (mimeType: string): Promise<{ data: Uint8Array, width: number, height: number }> =>
            loadImageData(`data:${mimeType};base64,${base64}`, false);

        tryLoad("image/png")
            .catch(() => tryLoad("image/jpeg"))
            .then(albumData => {
                if (myVersion !== albumLoadVersion) {
                    console.log("[Album] Stale load discarded (newer track arrived)");
                    return;
                }
                console.log("[Album] Decoded", albumData.width, "x", albumData.height, "— pushing to Wasm");
                currentAlbumData = albumData;
                if (app) {
                    app.update_album_art(albumData.data, albumData.width, albumData.height);
                }
            })
            .catch(err => console.error("[Album] Failed to decode thumbnail:", err));
    } catch (e) {
        console.error("[Album] Failed to parse livelyCurrentTrack data:", e);
    }
};

// isRemote=true sets crossOrigin (for http URLs); false skips it (for data: URLs)
async function loadImageData(url: string, isRemote = true): Promise<{ data: Uint8Array, width: number, height: number }> {
    return new Promise((resolve, reject) => {
        const img = new Image();
        if (isRemote) img.crossOrigin = "Anonymous";
        img.onload = () => {
            const c = document.createElement('canvas');
            c.width = img.width;
            c.height = img.height;
            const ctx = c.getContext('2d');
            if (!ctx) return reject("No 2d context");
            // Flip vertically for WebGL (bottom-left origin)
            ctx.translate(0, img.height);
            ctx.scale(1, -1);
            ctx.drawImage(img, 0, 0);
            const imageData = ctx.getImageData(0, 0, img.width, img.height);
            resolve({
                data: new Uint8Array(imageData.data.buffer),
                width: img.width,
                height: img.height
            });
        };
        img.onerror = () => reject(`Failed to load image: ${url.substring(0, 50)}`);
        img.src = url;
    });
}

async function loadApp() {
    if (!canvas || !logoData || !noiseData) return;
    
    let shaderFile = presets[props.preset] || presets[0];
    const response = await fetch(`/shaders/${shaderFile}`);
    const fragmentShaderSource = await response.text();

    app = new MatrixApp(
        canvas, 
        fragmentShaderSource, 
        logoData.data, logoData.width, logoData.height,
        noiseData.data, noiseData.width, noiseData.height
    );
    app.update_properties(props.red, props.green, props.blue, props.rainHighlights, props.intensity, props.distortion, props.dotSize, props.crtCurve ? 1.0 : 0.0, props.fallSpeed, props.noiseFluctuation, props.dotMode ? 1.0 : 0.0, props.lowPower ? 1.0 : 0.0);
    
    // Restore album data if it exists
    if (currentAlbumData) {
        app.update_album_art(currentAlbumData.data, currentAlbumData.width, currentAlbumData.height);
    }
}

async function start() {
    await init();

    canvas = document.getElementById('glcanvas') as HTMLCanvasElement;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;

    window.addEventListener('resize', () => {
        if (canvas) {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
            // Preset change will reload the context with new size. 
            // In a robust implementation, we'd add a resize method to Rust.
            loadApp();
        }
    });

    logoData = await loadImageData('/textures/logo.png');
    noiseData = await loadImageData('/textures/noise.png');

    await loadApp();

    let startTime = performance.now();

    function render(time: number) {
        if (app) {
            const timeInSeconds = (time - startTime) / 1000.0;
            app.update_audio(currentAudioData, timeInSeconds);
            app.render(timeInSeconds);
        }
        requestAnimationFrame(render);
    }

    requestAnimationFrame(render);
}

start().catch(console.error);
