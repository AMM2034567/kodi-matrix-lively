use glow::HasContext;

// ── Uniform locations ──────────────────────────────────────────────────────
struct Uniforms {
    i_time: Option<glow::UniformLocation>,
    i_channel: [Option<glow::UniformLocation>; 4],
    c_color: Option<glow::UniformLocation>,
    c_rain_highlights: Option<glow::UniformLocation>,
    c_dot_size: Option<glow::UniformLocation>,
    c_columns: Option<glow::UniformLocation>,
    c_intensity: Option<glow::UniformLocation>,
    c_distort_threshold: Option<glow::UniformLocation>,
    c_crt_curve: Option<glow::UniformLocation>,
    c_noise_fluctuation: Option<glow::UniformLocation>,
    c_low_power: Option<glow::UniformLocation>,
    i_album_position: Option<glow::UniformLocation>,
    i_album_rgb: Option<glow::UniformLocation>,
}

// ── Runtime properties (mirrors C++ CVisualizationMatrix fields) ───────────
#[derive(Clone, Copy)]
pub struct Props {
    pub color: [f32; 3],
    pub rain_highlights: f32,
    pub intensity: f32,
    pub distort_threshold: f32,
    pub dot_size: f32,
    pub crt_curve: f32,
    pub fall_speed: f32,
    pub noise_fluctuation: f32,
    pub dot_mode: bool,
    pub low_power: f32,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            color: [51.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0],
            rain_highlights: 70.0 * 0.016,
            intensity: 1.5,
            distort_threshold: 0.25,
            dot_size: 4.0,
            crt_curve: 0.0,
            fall_speed: 0.25,
            noise_fluctuation: 25.0,
            dot_mode: false,
            low_power: 0.0,
        }
    }
}

// ── Main renderer ──────────────────────────────────────────────────────────
pub struct MatrixRenderer {
    gl: glow::Context,
    program: glow::Program,
    width: i32,
    height: i32,
    u: Uniforms,

    audio_tex: glow::Texture,
    logo_tex: Option<glow::Texture>,
    noise_tex: Option<glow::Texture>,
    album_tex: Option<glow::Texture>,

    pub props: Props,

    last_album_change: f32,
    album_first_frame: bool,
    album_changed: bool,
    album_x: f32,
    album_y: f32,
    audio_dirty: bool, // C++ m_needsUpload — only upload texture when data changed
}

impl MatrixRenderer {
    pub fn new(
        gl: glow::Context,
        width: i32,
        height: i32,
        fragment_shader_source: &str,
        logo_pixels: Option<(&[u8], i32, i32)>,
        noise_pixels: Option<(&[u8], i32, i32)>,
    ) -> Result<Self, String> {
        unsafe {
            let program = Self::compile_program(&gl, width, height, fragment_shader_source)?;
            gl.use_program(Some(program));

            let u = Uniforms {
                i_time: gl.get_uniform_location(program, "iTime"),
                i_channel: [
                    gl.get_uniform_location(program, "iChannel0"),
                    gl.get_uniform_location(program, "iChannel1"),
                    gl.get_uniform_location(program, "iChannel2"),
                    gl.get_uniform_location(program, "iChannel3"),
                ],
                c_color: gl.get_uniform_location(program, "cColor"),
                c_rain_highlights: gl.get_uniform_location(program, "cRainHighlights"),
                c_intensity: gl.get_uniform_location(program, "cINTENSITY"),
                c_distort_threshold: gl.get_uniform_location(program, "cDistortThreshold"),
                c_crt_curve: gl.get_uniform_location(program, "cCrtCurve"),
                c_noise_fluctuation: gl.get_uniform_location(program, "cNoiseFluctuation"),
                c_low_power: gl.get_uniform_location(program, "dLowPower"),
                c_dot_size: gl.get_uniform_location(program, "cDotSize"),
                c_columns: gl.get_uniform_location(program, "cColumns"),
                i_album_position: gl.get_uniform_location(program, "iAlbumPosition"),
                i_album_rgb: gl.get_uniform_location(program, "iAlbumRGB"),
            };

            // Full-screen quad VBO
            let vertex_data: [f32; 16] = [
                -1.0,  1.0, 1.0, 1.0,
                 1.0,  1.0, 1.0, 1.0,
                 1.0, -1.0, 1.0, 1.0,
                -1.0, -1.0, 1.0, 1.0,
            ];
            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let bytes: &[u8] = core::slice::from_raw_parts(
                vertex_data.as_ptr() as *const u8,
                vertex_data.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 16, 0);

            // Audio texture (512×2 R8)
            let audio_tex = gl.create_texture().unwrap();
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(audio_tex));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            // Logo texture (channel 1)
            let logo_tex = logo_pixels.map(|(data, w, h)| {
                let tex = gl.create_texture().unwrap();
                gl.active_texture(glow::TEXTURE1);
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(data));
                tex
            });

            // Noise texture (channel 2, repeating)
            let noise_tex = noise_pixels.map(|(data, w, h)| {
                let tex = gl.create_texture().unwrap();
                gl.active_texture(glow::TEXTURE2);
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(data));
                tex
            });

            Ok(Self {
                gl,
                program,
                width,
                height,
                u,
                audio_tex,
                logo_tex,
                noise_tex,
                album_tex: None,
                props: Props::default(),
                last_album_change: 0.0,
                album_first_frame: true,
                album_changed: false,
                album_x: 0.0,
                album_y: 0.0,
                audio_dirty: false,
            })
        }
    }

    // ── Shader compilation ─────────────────────────────────────────────────
    fn compile_program(
        gl: &glow::Context,
        width: i32,
        height: i32,
        fragment_source: &str,
    ) -> Result<glow::Program, String> {
        unsafe {
            let vs_src = r#"#version 300 es
            precision highp float;
            in vec4 vertex;
            void main() { gl_Position = vertex; }
            "#;

            let header = Self::build_glsl_header(width, height);
            let full_fs = format!("{}\n{}", header, fragment_source);

            let program = gl.create_program().expect("Cannot create program");

            let vs = gl.create_shader(glow::VERTEX_SHADER).expect("Cannot create shader");
            gl.shader_source(vs, vs_src);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                return Err(gl.get_shader_info_log(vs));
            }

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).expect("Cannot create shader");
            gl.shader_source(fs, &full_fs);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                return Err(gl.get_shader_info_log(fs));
            }

            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.bind_attrib_location(program, 0, "vertex");
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            Ok(program)
        }
    }

    // ── GLSL header (C++ GatherDefines + fsCommonFunctionsNormal) ──────────
    fn build_glsl_header(width: i32, height: i32) -> String {
        let mut h = String::new();
        h.push_str("#version 300 es\n");
        h.push_str("precision highp float;\n");
        h.push_str("precision mediump int;\n");
        h.push_str("out vec4 FragColor;\n");

        h.push_str("const float cRNDSEED1 = 170.12;\n");
        h.push_str("const float cRNDSEED2 = 7572.1;\n");
        h.push_str("const float cMININTENSITY = 0.075;\n");
        h.push_str("const float cDISTORTFACTORX = 0.6;\n");
        h.push_str("const float cDISTORTFACTORY = 0.4;\n");
        h.push_str("const float cVIGNETTEINTENSITY = 0.05;\n");

        h.push_str("uniform float cDotSize;\n");
        h.push_str("uniform float cColumns;\n");
        h.push_str("uniform float cNoiseFluctuation;\n");
        h.push_str("uniform float dLowPower;\n");
        h.push_str(&format!("const vec2 cResolution = vec2({:.1}, {:.1});\n", width as f32, height as f32));

        h.push_str("uniform sampler2D iChannel0;\n");
        h.push_str("uniform sampler2D iChannel1;\n");
        h.push_str("uniform sampler2D iChannel2;\n");
        h.push_str("uniform sampler2D iChannel3;\n");
        h.push_str("uniform vec3 iAlbumPosition;\n");
        h.push_str("uniform vec3 iAlbumRGB;\n");
        h.push_str("#define dNoise\n");

        h.push_str("uniform float iTime;\n");
        h.push_str("uniform vec3 cColor;\n");
        h.push_str("uniform float cRainHighlights;\n");
        h.push_str("uniform float cINTENSITY;\n");
        h.push_str("uniform float cDistortThreshold;\n");
        h.push_str("uniform float cCrtCurve;\n");

        h.push_str(r#"
float h11(float p) {
  float r_low = fract(.13 * p + 217943.37373737 / (p + 0.31));
  float r_norm = fract(20.12345 + sin(p * cRNDSEED1) * cRNDSEED2);
  return mix(r_norm, r_low, dLowPower);
}

float waveform(vec2 uv) {
  float wave = texture(iChannel0, vec2(uv.x * .15 + .5, 0.75)).x;
  float wf_low = min(abs(uv.y * 20. + (wave - .5) * 10.), 0.5);
  float wf_norm = abs(smoothstep(.225, .275, wave * .5 + uv.y) - .5);
  return mix(wf_norm, wf_low, dLowPower);
}

#ifdef dNoise
float noise(vec2 gv) {
  float n_low = texture(iChannel2, vec2(gl_FragCoord.xy / (256.0 * cDotSize))).x;
  float n_norm = texture(iChannel2, (gv * .035431) + iTime * cNoiseFluctuation).x;
  return mix(n_norm, n_low, dLowPower);
}
#endif

vec3 bw2col(float bw, vec2 uv) {
  float d = length(fract(uv * cColumns) - .5);
  // Low power:  (basecolor*cColor + peakcolor) * bw
  float pk_low = .6 - d;
  float bc_low = .8 - d;
  vec3 r_low = (bc_low * cColor + pk_low) * bw;
  // Normal:     basecolor*cColor + peakcolor  (bw in smoothstep)
  float pk_norm = smoothstep(.35, .0, d) * bw;
  float bc_norm = smoothstep(.85, .0, d) * bw;
  vec3 r_norm = bc_norm * cColor + pk_norm;
  return mix(r_norm, r_low, dLowPower);
}

vec2 getUV() {
  vec2 uv = (gl_FragCoord.xy - .5 * cResolution.xy) / cResolution.y;
  vec2 crtUV = uv / (1.00 - length(uv * .1));
  return mix(uv, crtUV, cCrtCurve);
}
"#);
        h
    }

    // ── Album texture update ───────────────────────────────────────────────
    pub fn update_album_texture(&mut self, data: &[u8], w: i32, h: i32) {
        unsafe {
            if let Some(tex) = self.album_tex {
                self.gl.delete_texture(tex);
            }
            let tex = self.gl.create_texture().unwrap();
            self.gl.active_texture(glow::TEXTURE3);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl_tex_params(&self.gl, glow::CLAMP_TO_EDGE as i32);
            self.gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(data));
            self.album_tex = Some(tex);
            self.album_changed = true;
        }
    }

    // ── Called by MatrixApp when audio buffer content changed ────────────
    pub fn mark_audio_dirty(&mut self) {
        self.audio_dirty = true;
    }

    // ── Per-frame render (mirrors C++ RenderTo) ────────────────────────────
    //     time = real wall-clock time (for album animation, matching C++ logotimer)
    //     iTime = time * fallSpeed (for rain / effects speed)
    pub fn render(&mut self, time: f32, audio_data: &[u8]) {
        unsafe {
            self.gl.use_program(Some(self.program));
            self.gl.viewport(0, 0, self.width, self.height);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            let p = &self.props;
            let scaled = time * p.fall_speed;

            // ── 1. All uniforms ────────────────────────────────────────
            uni_1f(&self.gl, &self.u.i_time, scaled);
            uni_3f(&self.gl, &self.u.c_color, p.color[0], p.color[1], p.color[2]);
            uni_1f(&self.gl, &self.u.c_rain_highlights, p.rain_highlights);
            uni_1f(&self.gl, &self.u.c_intensity, p.intensity);
            uni_1f(&self.gl, &self.u.c_distort_threshold, p.distort_threshold);
            uni_1f(&self.gl, &self.u.c_crt_curve, p.crt_curve);

            let factor = if p.low_power > 0.5 { 0.0002 } else { 0.0004 };
            uni_1f(&self.gl, &self.u.c_noise_fluctuation,
                   p.noise_fluctuation * factor * 0.25 / p.fall_speed.max(0.001));
            uni_1f(&self.gl, &self.u.c_low_power, p.low_power);

            let dot = if p.dot_mode {
                p.dot_size
            } else if self.height <= 900 { 3.0 }
            else if self.height <= 1500 { 4.0 }
            else { 5.0 };
            uni_1f(&self.gl, &self.u.c_dot_size, dot);
            uni_1f(&self.gl, &self.u.c_columns, self.width as f32 / (dot * 2.0));

            // ── 2. Upload audio texture (only when dirty, like C++ m_needsUpload) ─
            //     When distort_threshold >= 0.5 (slider=100), zero out waveform row.
            if self.audio_dirty {
                let silence_wave = p.distort_threshold >= 0.499;
                let upload_data = if silence_wave {
                    let mut muted = [128u8; 1024];
                    muted[..512].copy_from_slice(&audio_data[..512]);
                    muted
                } else {
                    let mut buf = [0u8; 1024];
                    buf.copy_from_slice(audio_data);
                    buf
                };
                self.gl.active_texture(glow::TEXTURE0);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(self.audio_tex));
                self.gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::R8 as i32,
                                     512, 2, 0, glow::RED, glow::UNSIGNED_BYTE, Some(&upload_data));
                self.audio_dirty = false;
            }

            // ── 3. Album animation ───────────────────────────────────────────
            //     Position: iAlbumPosition.xy = offset, iAlbumPosition.z = scale.
            //     C++ uses z=2.0 which makes the album oversized; any offset > 0.2
            //     clips part of it.  We use z=1.0 + small offset range so the
            //     album is always comfortably visible on screen.
            if self.album_first_frame {
                self.album_first_frame = false;
                self.album_x = 0.5;
                self.album_y = 0.5;
                uni_3f(&self.gl, &self.u.i_album_position, 0.5, 0.5, 2.0);
            }

            if self.album_changed {
                self.last_album_change = time - 0.01;
                let r = (time * 1234.0).sin() * 10000.0;
                self.album_x = r.fract() * 0.4 + 0.3;
                self.album_y = (r * 7654.0).sin().fract() * 0.3 + 0.35;
                self.album_changed = false;
            }

            let d = (time - self.last_album_change) * 0.6;
            uni_3f(&self.gl, &self.u.i_album_rgb,
                   f32::max(d.sin(), 0.0) * 0.7,
                   f32::max((d - 1.0).sin(), 0.0) * 0.7,
                   f32::max((d - 2.0).sin(), 0.0) * 0.7);

            if time - self.last_album_change >= 10.0 {
                let r = (time * 1234.0).sin() * 10000.0;
                self.album_x = r.fract() * 0.4 + 0.3;
                self.album_y = (r * 7654.0).sin().fract() * 0.3 + 0.35;
                self.last_album_change = time;
            }
            uni_3f(&self.gl, &self.u.i_album_position, self.album_x, self.album_y, 2.0);

            // ── 4. Bind all textures every frame (C++: outside m_needsUpload) ─
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.audio_tex));
            uni_1i(&self.gl, &self.u.i_channel[0], 0);

            if let Some(tex) = self.logo_tex {
                self.gl.active_texture(glow::TEXTURE1);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                uni_1i(&self.gl, &self.u.i_channel[1], 1);
            }
            if let Some(tex) = self.noise_tex {
                self.gl.active_texture(glow::TEXTURE2);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                uni_1i(&self.gl, &self.u.i_channel[2], 2);
            }
            if let Some(tex) = self.album_tex {
                self.gl.active_texture(glow::TEXTURE3);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                uni_1i(&self.gl, &self.u.i_channel[3], 3);
            }

            // ── 5. Draw ────────────────────────────────────────────────
            self.gl.draw_arrays(glow::TRIANGLE_FAN, 0, 4);
        }
    }
}

// ── Free helpers ───────────────────────────────────────────────────────────
fn uni_1f(gl: &glow::Context, loc: &Option<glow::UniformLocation>, v: f32) {
    if let Some(ref l) = loc { unsafe { gl.uniform_1_f32(Some(l), v); } }
}
fn uni_3f(gl: &glow::Context, loc: &Option<glow::UniformLocation>, a: f32, b: f32, c: f32) {
    if let Some(ref l) = loc { unsafe { gl.uniform_3_f32(Some(l), a, b, c); } }
}
fn uni_1i(gl: &glow::Context, loc: &Option<glow::UniformLocation>, v: i32) {
    if let Some(ref l) = loc { unsafe { gl.uniform_1_i32(Some(l), v); } }
}
fn gl_tex_params(gl: &glow::Context, wrap: i32) {
    unsafe {
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);
    }
}
