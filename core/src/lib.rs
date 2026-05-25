mod renderer;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};
use renderer::MatrixRenderer;
use std::cell::RefCell;
use std::rc::Rc;

#[wasm_bindgen]
pub struct MatrixApp {
    renderer: Rc<RefCell<MatrixRenderer>>,
    audio_buffer: [u8; 1024],
    smoothed: [f32; 512],     // EMA-smoothed frequency bins
    smooth_bands: [f32; 4],   // EMA-smoothed band energies
}

#[wasm_bindgen]
impl MatrixApp {
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        fragment_shader_source: &str,
        logo_data: Option<js_sys::Uint8Array>, logo_w: i32, logo_h: i32,
        noise_data: Option<js_sys::Uint8Array>, noise_w: i32, noise_h: i32,
    ) -> Result<MatrixApp, JsValue> {
        let gl = canvas
            .get_context("webgl2")?
            .unwrap()
            .dyn_into::<WebGl2RenderingContext>()?;

        let glow_context = glow::Context::from_webgl2_context(gl);

        let width = canvas.width() as i32;
        let height = canvas.height() as i32;

        let logo_ref = logo_data.as_ref().map(|d| (d.to_vec(), logo_w, logo_h));
        let noise_ref = noise_data.as_ref().map(|d| (d.to_vec(), noise_w, noise_h));

        let logo = logo_ref.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h));
        let noise = noise_ref.as_ref().map(|(d, w, h)| (d.as_slice(), *w, *h));

        let renderer = MatrixRenderer::new(glow_context, width, height, fragment_shader_source, logo, noise)
            .map_err(|e| JsValue::from_str(e.as_str()))?;

        Ok(MatrixApp {
            renderer: Rc::new(RefCell::new(renderer)),
            audio_buffer: [0; 1024],
            smoothed: [0.0; 512],
            smooth_bands: [0.0; 4],
        })
    }

    // ── Audio buffer ──────────────────────────────────────────────────────
    //     Uses time-based phase (like C++ PCM, waveform evolves with real time)
    //     at frequencies high enough to avoid visible flicker (> 60 Hz).
    #[wasm_bindgen]
    pub fn update_audio(&mut self, frequency_data: &[f32], time: f32) {
        const SMOOTH: f32 = 0.3;       // per-bin EMA — lower = faster response
        const BAND_SMOOTH: f32 = 0.15;  // band-energy EMA — lower = less lag

        // Step 1 — EMA smooth raw bins
        for i in 0..128 {
            let raw = if i < frequency_data.len() { frequency_data[i] } else { 0.0 };
            for j in 0..4 {
                let idx = i * 4 + j;
                if idx < 512 {
                    self.smoothed[idx] = SMOOTH * self.smoothed[idx] + (1.0 - SMOOTH) * raw;
                }
            }
        }

        // Step 2 — Band energies
        let (mut bass, mut mid, mut high) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..512 {
            let v = self.smoothed[i];
            if i < 64 { bass += v; } else if i < 224 { mid += v; } else { high += v; }
        }
        bass = (bass / 64.0).sqrt();
        mid = (mid / 160.0).sqrt();
        high = (high / 288.0).sqrt();
        let overall = ((bass + mid + high) / 3.0).sqrt();

        // Step 3 — EMA on band energies
        self.smooth_bands[0] = BAND_SMOOTH * self.smooth_bands[0] + (1.0 - BAND_SMOOTH) * bass;
        self.smooth_bands[1] = BAND_SMOOTH * self.smooth_bands[1] + (1.0 - BAND_SMOOTH) * mid;
        self.smooth_bands[2] = BAND_SMOOTH * self.smooth_bands[2] + (1.0 - BAND_SMOOTH) * high;
        self.smooth_bands[3] = BAND_SMOOTH * self.smooth_bands[3] + (1.0 - BAND_SMOOTH) * overall;

        // Step 4 — Fill buffer with time-based phase
        //     Single composite waveform with full-range amplitude.
        //     amp directly scales the sine → texture spans 0-255 when amp→1.0.
        let phase = time as f64 * 30.0;
        let composite = (self.smooth_bands[0] + self.smooth_bands[1] + self.smooth_bands[2]) / 3.0
                        + self.smooth_bands[3] * 0.5;
        let amp = composite.min(1.0) * 0.5;

        for i in 0..512 {
            let t = i as f32 / 512.0;
            self.audio_buffer[i] = (self.smoothed[i] * 255.0).clamp(0.0, 255.0) as u8;

            // Pseudo-random noise (sin hash at two frequencies) mimics real PCM's
            // sample-to-sample randomness, making the waveform look less "smooth"
            // and therefore less visually intrusive in waveform/envelope presets.
            let raw = (t as f64 * 10000.0 + phase * 100.0).sin() * 100000.0;
            let noise = (raw - raw.floor()) as f32; // 0..1
            let s = (noise * 2.0 - 1.0) * amp;
            self.audio_buffer[i + 512] = ((128.0 + s * 255.0).clamp(0.0, 255.0)) as u8;
        }

        // Always dirty — like C++, waveform naturally evolves each frame
        self.renderer.borrow_mut().mark_audio_dirty();
    }

    #[wasm_bindgen]
    pub fn update_properties(
        &mut self,
        red: f32, green: f32, blue: f32,
        rain_highlights: f32,
        intensity: f32,
        distortion: f32,
        dot_size: f32,
        crt_curve: f32,
        fall_speed: f32,
        noise_fluctuation: f32,
        dot_mode: f32,
        low_power: f32,
    ) {
        let mut r = self.renderer.borrow_mut();
        r.props.color = [red / 255.0, green / 255.0, blue / 255.0];
        r.props.rain_highlights = rain_highlights * 0.016;
        r.props.intensity = intensity;
        r.props.distort_threshold = distortion * 0.005;
        r.props.dot_size = dot_size;
        r.props.crt_curve = crt_curve;
        r.props.fall_speed = fall_speed * 0.01;
        r.props.noise_fluctuation = noise_fluctuation;
        r.props.dot_mode = dot_mode > 0.5;
        r.props.low_power = low_power;
    }

    #[wasm_bindgen]
    pub fn update_album_art(&mut self, data: &[u8], w: i32, h: i32) {
        self.renderer.borrow_mut().update_album_texture(data, w, h);
    }

    #[wasm_bindgen]
    pub fn render(&mut self, time: f32) {
        self.renderer.borrow_mut().render(time, &self.audio_buffer);
    }
}
