//! Linhas Conectoras - nucleo sem dependencias do After Effects:
//! geometria (arcos / espirais sem cruzamento), sorteios deterministicos,
//! deteccao de "letras" pelo alpha, montagem da cena e rasterizacao com AA.

use std::f64::consts::PI;

// ------------------------------------------------------------------
// Vetores e matrizes
// ------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}
impl V2 {
    pub const fn new(x: f64, y: f64) -> Self { Self { x, y } }
    pub fn len(self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
}
impl std::ops::Add for V2 { type Output = V2; fn add(self, o: V2) -> V2 { V2::new(self.x + o.x, self.y + o.y) } }
impl std::ops::Sub for V2 { type Output = V2; fn sub(self, o: V2) -> V2 { V2::new(self.x - o.x, self.y - o.y) } }
impl std::ops::Mul<f64> for V2 { type Output = V2; fn mul(self, s: f64) -> V2 { V2::new(self.x * s, self.y * s) } }

#[derive(Clone, Copy, Debug, Default)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Matriz 4x4 na convencao do After Effects (vetor-linha: p' = p * M,
/// translacao em m[3][0..2]).
#[derive(Clone, Copy, Debug)]
pub struct M4(pub [[f64; 4]; 4]);

impl M4 {
    pub fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() { row[i] = 1.0; }
        M4(m)
    }
    pub fn from_slice(s: &[f64]) -> Self {
        let mut m = [[0.0; 4]; 4];
        for i in 0..4 { for j in 0..4 { m[i][j] = s[i * 4 + j]; } }
        M4(m)
    }
    pub fn xform(&self, p: V3) -> V3 {
        let m = &self.0;
        let x = p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0] + m[3][0];
        let y = p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1] + m[3][1];
        let z = p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2] + m[3][2];
        let w = p.x * m[0][3] + p.y * m[1][3] + p.z * m[2][3] + m[3][3];
        if w.abs() > 1e-12 && (w - 1.0).abs() > 1e-12 { V3 { x: x / w, y: y / w, z: z / w } } else { V3 { x, y, z } }
    }
    /// Inversa por Gauss-Jordan.
    pub fn invert(&self) -> Option<M4> {
        let mut a = [[0.0f64; 8]; 4];
        for i in 0..4 {
            for j in 0..4 { a[i][j] = self.0[i][j]; }
            a[i][4 + i] = 1.0;
        }
        for c in 0..4 {
            let mut piv = c;
            for r in c + 1..4 { if a[r][c].abs() > a[piv][c].abs() { piv = r; } }
            if a[piv][c].abs() < 1e-14 { return None; }
            a.swap(c, piv);
            let d = a[c][c];
            for j in 0..8 { a[c][j] /= d; }
            for r in 0..4 {
                if r == c { continue; }
                let f = a[r][c];
                if f != 0.0 { for j in 0..8 { a[r][j] -= f * a[c][j]; } }
            }
        }
        let mut out = [[0.0; 4]; 4];
        for i in 0..4 { for j in 0..4 { out[i][j] = a[i][j + 4]; } }
        Some(M4(out))
    }
}

/// Camera: projeta um ponto do mundo 3D da comp para a tela da comp.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub world_to_cam: M4,
    pub zoom: f64,
    pub cx: f64,
    pub cy: f64,
}
impl Camera {
    /// Camera padrao do AE: no centro da comp, recuada pela distancia do plano de imagem.
    pub fn default_for(comp_w: f64, comp_h: f64, zoom: f64) -> Self {
        let mut t = M4::identity();
        t.0[3][0] = -comp_w * 0.5;
        t.0[3][1] = -comp_h * 0.5;
        t.0[3][2] = zoom;
        Camera { world_to_cam: t, zoom, cx: comp_w * 0.5, cy: comp_h * 0.5 }
    }
    pub fn project(&self, w: V3) -> V2 {
        let c = self.world_to_cam.xform(w);
        let z = c.z.max(1e-3); // atras da camera: evita explodir
        V2::new(c.x * self.zoom / z + self.cx, c.y * self.zoom / z + self.cy)
    }
}

/// Transform de um layer (para a tela da comp).
#[derive(Clone, Copy, Debug)]
pub struct LayerXf {
    pub to_world: M4,
    pub is_3d: bool,
}
impl LayerXf {
    pub fn to_screen(&self, cam: &Camera, x: f64, y: f64, z: f64) -> V2 {
        let w = self.to_world.xform(V3 { x, y, z });
        if self.is_3d { cam.project(w) } else { V2::new(w.x, w.y) }
    }
}

// ------------------------------------------------------------------
// Sorteios deterministicos
// ------------------------------------------------------------------
#[inline]
pub fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
pub fn key(a: u64, b: u64, c: u64, d: u64) -> u64 { mix64(mix64(mix64(mix64(a) ^ b) ^ c) ^ d) }

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self { Rng(mix64(seed)) }
    pub fn next(&mut self) -> f64 {
        self.0 = mix64(self.0);
        (self.0 >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
}

#[inline]
pub fn clamp01(v: f64) -> f64 { v.clamp(0.0, 1.0) }

/// Mesmo `ease()` das expressions do AE (curva cosseno).
pub fn ease_range(t: f64, a: f64, b: f64) -> f64 {
    if b - a < 1e-12 { return if t >= b { 1.0 } else { 0.0 }; }
    let u = clamp01((t - a) / (b - a));
    0.5 - 0.5 * (PI * u).cos()
}

// ------------------------------------------------------------------
// Cor
// ------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

pub fn rgb_to_hsl(c: Rgb) -> (f64, f64, f64) {
    let mx = c.r.max(c.g).max(c.b);
    let mn = c.r.min(c.g).min(c.b);
    let l = (mx + mn) / 2.0;
    if mx - mn < 1e-12 { return (0.0, 0.0, l); }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == c.r {
        (c.g - c.b) / d + if c.g < c.b { 6.0 } else { 0.0 }
    } else if mx == c.g {
        (c.b - c.r) / d + 2.0
    } else {
        (c.r - c.g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hue2rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 0.5 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    if s < 1e-12 { return Rgb { r: l, g: l, b: l }; }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    Rgb { r: hue2rgb(p, q, h + 1.0 / 3.0), g: hue2rgb(p, q, h), b: hue2rgb(p, q, h - 1.0 / 3.0) }
}

// ------------------------------------------------------------------
// Curvas
// ------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub struct Bez {
    pub p0: V2,
    pub c0: V2,
    pub c1: V2,
    pub p1: V2,
}
impl Bez {
    pub fn at(&self, u: f64) -> V2 {
        let a = 1.0 - u;
        let (w0, w1, w2, w3) = (a * a * a, 3.0 * a * a * u, 3.0 * a * u * u, u * u * u);
        V2::new(
            w0 * self.p0.x + w1 * self.c0.x + w2 * self.c1.x + w3 * self.p1.x,
            w0 * self.p0.y + w1 * self.c0.y + w2 * self.c1.y + w3 * self.p1.y,
        )
    }
    pub fn map(&self, f: &impl Fn(V2) -> V2) -> Bez { Bez { p0: f(self.p0), c0: f(self.c0), c1: f(self.c1), p1: f(self.p1) } }
}

#[derive(Clone, Copy, Debug)]
pub struct CurveParams {
    pub curv: f64,      // Curvatura / 100
    pub vari: f64,      // Variacao / 100
    pub up: bool,       // Arco para cima
    pub inv: bool,      // Inverter
    pub no_cross: bool, // Sem cruzamento (espiral)
    pub comp_half: f64, // max(w, h) / 2 da comp
}
impl Default for CurveParams {
    fn default() -> Self { Self { curv: 0.3, vari: 0.4, up: true, inv: false, no_cross: false, comp_half: 960.0 } }
}

/// Arco "rota de voo" de A ate B (um cubic).
pub fn arc_curve(a: V2, b: V2, rv: f64, p: &CurveParams, out: &mut Vec<Bez>) {
    let d = b - a;
    let l = d.len();
    if l < 1e-3 {
        out.push(Bez { p0: a, c0: a, c1: b, p1: b });
        return;
    }
    let (mut px, mut py) = (-d.y / l, d.x / l);
    if p.up && (py > 0.0 || (py == 0.0 && px > 0.0)) { px = -px; py = -py; }
    if p.inv { px = -px; py = -py; }
    let h = l * p.curv * (1.0 + rv * p.vari).max(0.0);
    let k = h * 4.0 / 3.0;
    let out_t = V2::new(d.x * 0.25 + px * k, d.y * 0.25 + py * k);
    let in_t = V2::new(-d.x * 0.25 + px * k, -d.y * 0.25 + py * k);
    out.push(Bez { p0: a, c0: a + out_t, c1: b + in_t, p1: b });
}

/// Espiral "sem cruzamento" em volta do centro C (rotacoes da mesma curva),
/// terminando no raio r_end. Todas as linhas de um mesmo null nunca se cruzam.
pub fn spiral_curve(a: V2, c: V2, r_end: f64, p: &CurveParams, out: &mut Vec<Bez>) {
    let (vx, vy) = (a.x - c.x, a.y - c.y);
    let r0 = (vx * vx + vy * vy).sqrt();
    let re = r_end.min(r0 * 0.5);
    let dd = p.curv * 2.0 * if p.inv { -1.0 } else { 1.0 };
    let dr_k = dd / p.comp_half;
    let base = vy.atan2(vx) - dr_k * r0;
    const NV: usize = 12;
    let dr = (re - r0) / NV as f64;
    let mut prev = V2::default();
    let mut prev_t = V2::default();
    for i in 0..=NV {
        let r = r0 + dr * i as f64;
        let an = base + dr_k * r;
        let (si, co) = an.sin_cos();
        let pt = V2::new(c.x + co * r, c.y + si * r);
        let t = V2::new((co - r * dr_k * si) * dr / 3.0, (si + r * dr_k * co) * dr / 3.0);
        if i > 0 { out.push(Bez { p0: prev, c0: prev + prev_t, c1: pt - t, p1: pt }); }
        prev = pt;
        prev_t = t;
    }
}

/// Achata curvas numa polilinha (segmentos de ~2 px).
pub fn flatten(bz: &[Bez], pts: &mut Vec<V2>) {
    pts.clear();
    for (i, b) in bz.iter().enumerate() {
        let est = (b.c0 - b.p0).len() + (b.c1 - b.c0).len() + (b.p1 - b.c1).len();
        let n = ((est / 2.0).ceil() as usize).clamp(1, 256);
        if i == 0 { pts.push(b.p0); }
        for k in 1..=n { pts.push(b.at(k as f64 / n as f64)); }
    }
}

// ------------------------------------------------------------------
// Rasterizador com antialiasing. Cobertura de UMA forma por vez (max por
// pixel) e composicao do incremento: sem "nos" onde segmentos se sobrepoem.
// ------------------------------------------------------------------
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    cov: Vec<f32>,
    stamp: Vec<u32>,
    cur: u32,
    /// RGBA premultiplicado em float (0..1)
    pub rgba: Vec<f32>,
    col: (f64, f64, f64, f64),
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, cov: vec![0.0; w * h], stamp: vec![0; w * h], cur: 0, rgba: vec![0.0; w * h * 4], col: (1.0, 1.0, 1.0, 1.0) }
    }
    pub fn begin(&mut self, c: Rgb, opacity: f64) {
        self.cur = self.cur.wrapping_add(1);
        if self.cur == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.cur = 1;
        }
        self.col = (c.r, c.g, c.b, opacity);
    }
    #[inline]
    pub fn plot(&mut self, x: i64, y: i64, c: f64) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || c <= 0.0 { return; }
        let i = y as usize * self.w + x as usize;
        let c = c.min(1.0);
        let old = if self.stamp[i] == self.cur { self.cov[i] as f64 } else { 0.0 };
        if self.stamp[i] != self.cur { self.stamp[i] = self.cur; self.cov[i] = 0.0; }
        if c <= old { return; }
        self.cov[i] = c as f32;
        // incremento exato do "over" com a mesma cor: a' = (c - old) / (1 - old)
        let mut a = if old >= 0.999_999 { 0.0 } else { (c - old) / (1.0 - old) };
        a *= self.col.3;
        let p = &mut self.rgba[i * 4..i * 4 + 4];
        let inv = 1.0 - a;
        p[0] = (self.col.0 * a + p[0] as f64 * inv) as f32;
        p[1] = (self.col.1 * a + p[1] as f64 * inv) as f32;
        p[2] = (self.col.2 * a + p[2] as f64 * inv) as f32;
        p[3] = (a + p[3] as f64 * inv) as f32;
    }

    /// Segmento com espessura (capsula).
    pub fn segment(&mut self, a: V2, b: V2, width: f64) {
        let hw = width.max(1.0) * 0.5;
        let fade = if width < 1.0 { width } else { 1.0 };
        let x0 = ((a.x.min(b.x) - hw - 1.0).floor() as i64).max(0);
        let x1 = ((a.x.max(b.x) + hw + 1.0).ceil() as i64).min(self.w as i64 - 1);
        let y0 = ((a.y.min(b.y) - hw - 1.0).floor() as i64).max(0);
        let y1 = ((a.y.max(b.y) + hw + 1.0).ceil() as i64).min(self.h as i64 - 1);
        let (vx, vy) = (b.x - a.x, b.y - a.y);
        let vv = vx * vx + vy * vy;
        for y in y0..=y1 {
            let py = y as f64 + 0.5;
            for x in x0..=x1 {
                let px = x as f64 + 0.5;
                let t = if vv > 1e-12 { (((px - a.x) * vx + (py - a.y) * vy) / vv).clamp(0.0, 1.0) } else { 0.0 };
                let (dx, dy) = (px - a.x - t * vx, py - a.y - t * vy);
                let c = hw + 0.5 - (dx * dx + dy * dy).sqrt();
                if c > 0.0 { self.plot(x, y, c.min(1.0) * fade); }
            }
        }
    }

    /// Polilinha cortada no comprimento `frac` (Trim End).
    pub fn polyline(&mut self, pts: &[V2], width: f64, frac: f64) {
        if pts.len() < 2 || frac <= 0.0 { return; }
        let total: f64 = pts.windows(2).map(|w| (w[1] - w[0]).len()).sum();
        let limit = total * clamp01(frac);
        let mut acc = 0.0;
        for w in pts.windows(2) {
            let sl = (w[1] - w[0]).len();
            if acc + sl >= limit {
                let u = if sl > 1e-12 { (limit - acc) / sl } else { 0.0 };
                let e = w[0] + (w[1] - w[0]) * u;
                self.segment(w[0], e, width);
                return;
            }
            self.segment(w[0], w[1], width);
            acc += sl;
        }
    }

    /// Marcadores: 1 circulo, 2 circulo cheio, 3 quadrado, 4 cruz, 5 losango.
    pub fn marker(&mut self, shape: i32, c: V2, size: f64, width: f64) {
        if size <= 0.01 || shape <= 0 { return; }
        let r = size * 0.5;
        let hw = width.max(0.5) * 0.5;
        if shape == 4 {
            self.segment(V2::new(c.x - r, c.y), V2::new(c.x + r, c.y), width);
            self.segment(V2::new(c.x, c.y - r), V2::new(c.x, c.y + r), width);
            return;
        }
        let ext = r + hw + 2.0;
        let x0 = ((c.x - ext).floor() as i64).max(0);
        let x1 = ((c.x + ext).ceil() as i64).min(self.w as i64 - 1);
        let y0 = ((c.y - ext).floor() as i64).max(0);
        let y1 = ((c.y + ext).ceil() as i64).min(self.h as i64 - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f64 + 0.5 - c.x, y as f64 + 0.5 - c.y);
                let cv = match shape {
                    1 => hw + 0.5 - ((dx * dx + dy * dy).sqrt() - r).abs(),
                    2 => r + 0.5 - (dx * dx + dy * dy).sqrt(),
                    3 => hw + 0.5 - (dx.abs().max(dy.abs()) - r).abs(),
                    5 => hw + 0.5 - ((dx.abs() + dy.abs() - r) * std::f64::consts::FRAC_1_SQRT_2).abs(),
                    _ => 0.0,
                };
                if cv > 0.0 { self.plot(x, y, cv.min(1.0)); }
            }
        }
    }
}

// ------------------------------------------------------------------
// Contorno pelo alpha: rotula "letras" (componentes conectados) e coleta
// os pixels de borda de cada uma, em ordem de varredura (estavel).
// ------------------------------------------------------------------
pub fn extract_letters(w: usize, h: usize, stride: usize, thresh: u8, min_edge: usize,
                       alpha_at: impl Fn(usize, usize) -> u8) -> Vec<Vec<V2>> {
    if w == 0 || h == 0 { return Vec::new(); }
    let stride = stride.max(1);
    let gw = w.div_ceil(stride);
    let gh = h.div_ceil(stride);
    let n = gw * gh;
    let mut inside = vec![false; n];
    for gy in 0..gh {
        for gx in 0..gw {
            let x = (gx * stride + stride / 2).min(w - 1);
            let y = (gy * stride + stride / 2).min(h - 1);
            inside[gy * gw + gx] = alpha_at(x, y) >= thresh;
        }
    }
    // union-find, 8-conectividade
    let mut par: Vec<usize> = (0..n).collect();
    fn find(par: &mut [usize], mut a: usize) -> usize {
        while par[a] != a { par[a] = par[par[a]]; a = par[a]; }
        a
    }
    fn unite(par: &mut [usize], a: usize, b: usize) {
        let (a, b) = (find(par, a), find(par, b));
        if a != b { if a < b { par[b] = a; } else { par[a] = b; } }
    }
    for gy in 0..gh {
        for gx in 0..gw {
            let i = gy * gw + gx;
            if !inside[i] { continue; }
            if gx > 0 && inside[i - 1] { unite(&mut par, i, i - 1); }
            if gy > 0 {
                let u = i - gw;
                if inside[u] { unite(&mut par, i, u); }
                if gx > 0 && inside[u - 1] { unite(&mut par, i, u - 1); }
                if gx + 1 < gw && inside[u + 1] { unite(&mut par, i, u + 1); }
            }
        }
    }
    let mut root_map = vec![usize::MAX; n];
    let mut edges: Vec<Vec<V2>> = Vec::new();
    for gy in 0..gh {
        for gx in 0..gw {
            let i = gy * gw + gx;
            if !inside[i] { continue; }
            let edge = gx == 0 || gy == 0 || gx + 1 == gw || gy + 1 == gh
                || !inside[i - 1] || !inside[i + 1] || !inside[i - gw] || !inside[i + gw];
            if !edge { continue; }
            let r = find(&mut par, i);
            if root_map[r] == usize::MAX { root_map[r] = edges.len(); edges.push(Vec::new()); }
            edges[root_map[r]].push(V2::new((gx * stride) as f64 + stride as f64 * 0.5, (gy * stride) as f64 + stride as f64 * 0.5));
        }
    }
    edges.into_iter().filter(|e| e.len() >= min_edge.max(1)).collect()
}

// ------------------------------------------------------------------
// Cena
// ------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct SceneParams {
    pub lines_min: i32,
    pub lines_max: i32,
    pub letters_per_null: i32,
    pub curve: CurveParams,
    pub disp: f64,
    pub progress: f64,  // 0..1
    pub time_rand: f64, // 0..0.95
    pub seed: u64,
    pub color: Rgb,
    pub opacity: f64,
    pub hue_var: f64, // graus
    pub bri_var: f64, // 0..1
    pub width: f64,
    pub width_var: f64,
    pub mark_shape: i32, // 0 = nenhum
    pub mark_size: f64,
    pub mark_size_var: f64,
    pub mark_width: f64,
    pub mark_with_line: bool,
    pub nn_on: bool,
    pub nn_neighbors: i32,
    pub nn_per_pair: i32,
}
impl Default for SceneParams {
    fn default() -> Self {
        Self {
            lines_min: 3, lines_max: 6, letters_per_null: 3, curve: CurveParams::default(), disp: 10.0,
            progress: 1.0, time_rand: 0.5, seed: 1, color: Rgb { r: 0.35, g: 0.8, b: 1.0 }, opacity: 1.0,
            hue_var: 20.0, bri_var: 0.25, width: 2.0, width_var: 0.4, mark_shape: 1, mark_size: 8.0,
            mark_size_var: 0.4, mark_width: 1.5, mark_with_line: true, nn_on: false, nn_neighbors: 2, nn_per_pair: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawItem {
    pub curve: Vec<Bez>,
    pub color: Rgb,
    pub width: f64,
    pub frac: f64,        // Trim End 0..1
    pub marker_size: f64, // ja com o "pop"
}

#[derive(Clone, Debug, Default)]
pub struct ScreenLetter {
    pub edge: Vec<V2>, // pontos de borda ja na tela (comp)
    pub center: V2,
}
impl ScreenLetter {
    pub fn from_edge(edge: Vec<V2>) -> Self {
        let n = edge.len().max(1) as f64;
        let (sx, sy) = edge.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
        ScreenLetter { center: V2::new(sx / n, sy / n), edge }
    }
}

fn varied_color(p: &SceneParams, r: &mut Rng) -> Rgb {
    let (mut h, s, l) = rgb_to_hsl(p.color);
    h += (r.next() * 2.0 - 1.0) * p.hue_var / 360.0;
    h -= h.floor();
    let l = clamp01(l * (1.0 + (r.next() * 2.0 - 1.0) * p.bri_var));
    hsl_to_rgb(h, s, l)
}

fn finish_item(curve: Vec<Bez>, p: &SceneParams, r: &mut Rng, s_start: f64) -> DrawItem {
    let rr = p.time_rand.clamp(0.0, 0.95);
    let frac = ease_range(p.progress, s_start, s_start + (1.0 - rr));
    let color = varied_color(p, r);
    let width = (p.width * (1.0 + (r.next() * 2.0 - 1.0) * p.width_var)).max(0.0);
    let ms = p.mark_size * (1.0 + (r.next() * 2.0 - 1.0) * p.mark_size_var);
    let pop = if p.mark_with_line { ease_range(frac, 0.9, 1.0) } else { 1.0 };
    let marker_size = if p.mark_shape > 0 { (ms * pop).max(0.0) } else { 0.0 };
    DrawItem { curve, color, width, frac, marker_size }
}

fn d2(a: V2, b: V2) -> f64 { let d = a - b; d.x * d.x + d.y * d.y }

/// Monta todas as linhas (espaco da comp).
pub fn build_scene(letters: &[ScreenLetter], nulls: &[V2], p: &SceneParams) -> Vec<DrawItem> {
    let mut out = Vec::new();
    let (nl, nn) = (letters.len(), nulls.len());
    if nn == 0 { return out; }
    let rr = p.time_rand.clamp(0.0, 0.95);

    // distribuicao: null mais proximo + minimo de letras por null
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); nn];
    for (li, l) in letters.iter().enumerate() {
        let best = (0..nn).min_by(|&a, &b| d2(l.center, nulls[a]).total_cmp(&d2(l.center, nulls[b]))).unwrap_or(0);
        buckets[best].push(li);
    }
    let min_per = (p.letters_per_null.max(0) as usize).min(nl);
    for ni in 0..nn {
        if buckets[ni].len() >= min_per { continue; }
        let mut order: Vec<usize> = (0..nl).collect();
        order.sort_by(|&a, &b| d2(letters[a].center, nulls[ni]).total_cmp(&d2(letters[b].center, nulls[ni])));
        for li in order {
            if buckets[ni].len() >= min_per { break; }
            if !buckets[ni].contains(&li) { buckets[ni].push(li); }
        }
    }

    // linhas letra -> null
    for (ni, bucket) in buckets.iter().enumerate() {
        for &li in bucket {
            let l = &letters[li];
            if l.edge.is_empty() { continue; }
            let mut rc = Rng::new(key(p.seed, ni as u64 + 1, li as u64 + 1, 0xC0FFEE));
            let span = (p.lines_max - p.lines_min).max(0) + 1;
            let count = p.lines_min.max(0) + (rc.next() * span as f64) as i32;
            for j in 0..count {
                let mut r = Rng::new(key(p.seed, ni as u64 + 1, li as u64 + 1, j as u64 + 1));
                let idx = ((r.next() * l.edge.len() as f64) as usize).min(l.edge.len() - 1);
                let rv = r.next() * 2.0 - 1.0;
                let dang = r.next() * 2.0 * PI;
                let drad = r.next().sqrt() * p.disp;
                let s_start = r.next() * rr;
                let a = l.edge[idx];
                let b0 = nulls[ni];
                let b = V2::new(b0.x + dang.cos() * drad, b0.y + dang.sin() * drad);
                let mut curve = Vec::new();
                if p.curve.no_cross { spiral_curve(a, b0, drad, &p.curve, &mut curve); } else { arc_curve(a, b, rv, &p.curve, &mut curve); }
                out.push(finish_item(curve, p, &mut r, s_start));
            }
        }
    }

    // linhas null -> null
    if p.nn_on && nn > 1 {
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for a in 0..nn {
            let mut o: Vec<usize> = (0..nn).filter(|&b| b != a).collect();
            o.sort_by(|&x, &y| d2(nulls[x], nulls[a]).total_cmp(&d2(nulls[y], nulls[a])));
            let k = if p.nn_neighbors > 0 { (p.nn_neighbors as usize).min(o.len()) } else { o.len() };
            for &b in &o[..k] {
                let pr = (a.min(b), a.max(b));
                if !pairs.contains(&pr) { pairs.push(pr); }
            }
        }
        if p.curve.no_cross {
            let orient = |a: V2, b: V2, c: V2| {
                let v = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                if v > 1e-9 { 1 } else if v < -1e-9 { -1 } else { 0 }
            };
            pairs.sort_by(|x, y| d2(nulls[x.0], nulls[x.1]).total_cmp(&d2(nulls[y.0], nulls[y.1])));
            let mut kept: Vec<(usize, usize)> = Vec::new();
            for &pp in &pairs {
                let ok = kept.iter().all(|&q| {
                    if pp.0 == q.0 || pp.0 == q.1 || pp.1 == q.0 || pp.1 == q.1 { return true; }
                    let (a, b, c, d) = (nulls[pp.0], nulls[pp.1], nulls[q.0], nulls[q.1]);
                    !(orient(a, b, c) * orient(a, b, d) < 0 && orient(c, d, a) * orient(c, d, b) < 0)
                });
                if ok { kept.push(pp); }
            }
            pairs = kept;
        }
        let mut cp = p.curve;
        cp.no_cross = false;
        for &(pa, pb) in &pairs {
            for j in 0..p.nn_per_pair.max(1) {
                let mut r = Rng::new(key(p.seed, 100_000 + pa as u64, pb as u64 + 1, j as u64 + 1));
                let rv = r.next() * 2.0 - 1.0;
                let (da, dr) = (r.next() * 2.0 * PI, r.next().sqrt() * p.disp);
                let (db, er) = (r.next() * 2.0 * PI, r.next().sqrt() * p.disp);
                let s_start = r.next() * rr;
                let a = V2::new(nulls[pa].x + da.cos() * dr, nulls[pa].y + da.sin() * dr);
                let b = V2::new(nulls[pb].x + db.cos() * er, nulls[pb].y + db.sin() * er);
                let mut curve = Vec::new();
                arc_curve(a, b, rv, &cp, &mut curve);
                out.push(finish_item(curve, p, &mut r, s_start));
            }
        }
    }
    out
}

/// Desenha a cena: marcadores primeiro (ficam ABAIXO das linhas), depois linhas.
/// `to_buf` converte ponto da comp -> pixel do buffer; `scale` px comp -> px buffer.
pub fn render_scene(items: &[DrawItem], p: &SceneParams, to_buf: &impl Fn(V2) -> V2, scale: f64, cv: &mut Canvas) {
    for it in items {
        if it.marker_size <= 0.0 { continue; }
        let Some(last) = it.curve.last() else { continue };
        cv.begin(it.color, p.opacity);
        cv.marker(p.mark_shape, to_buf(last.p1), it.marker_size * scale, p.mark_width * scale);
    }
    let mut pts = Vec::new();
    let mut tb = Vec::new();
    for it in items {
        if it.frac <= 0.0 || it.curve.is_empty() { continue; }
        tb.clear();
        tb.extend(it.curve.iter().map(|b| b.map(to_buf)));
        flatten(&tb, &mut pts);
        cv.begin(it.color, p.opacity);
        cv.polyline(&pts, it.width * scale, it.frac);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_alpha(x: usize, y: usize) -> u8 {
        let (dx, dy) = (x as f64 - 640.0, y as f64 - 360.0);
        let r = (dx * dx + dy * dy).sqrt();
        let in_rect = |x0, y0, x1, y1| x >= x0 && x < x1 && y >= y0 && y < y1;
        if in_rect(380, 300, 470, 420) || (r > 40.0 && r < 60.0) || in_rect(810, 300, 830, 420) || in_rect(830, 300, 900, 320) { 255 } else { 0 }
    }

    #[test]
    fn letras_detectadas() {
        let l = extract_letters(1280, 720, 2, 128, 8, fake_alpha);
        assert_eq!(l.len(), 3);
    }

    #[test]
    fn espiral_nao_cruza() {
        fn o(a: V2, b: V2, c: V2) -> f64 { ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).signum() }
        let p = CurveParams { curv: 1.0, no_cross: true, comp_half: 960.0, ..Default::default() };
        let mut rng = Rng::new(7);
        let c = V2::new(960.0, 540.0);
        let lines: Vec<Vec<V2>> = (0..25).map(|_| {
            let a = V2::new(rng.next() * 1920.0, rng.next() * 1080.0);
            let mut bz = Vec::new();
            spiral_curve(a, c, rng.next().sqrt() * 15.0, &p, &mut bz);
            let mut pts = Vec::new();
            flatten(&bz, &mut pts);
            pts
        }).collect();
        let mut x = 0;
        for i in 0..lines.len() { for j in i + 1..lines.len() {
            for s in lines[i].windows(2) { for t in lines[j].windows(2) {
                if o(s[0], s[1], t[0]) * o(s[0], s[1], t[1]) < 0.0 && o(t[0], t[1], s[0]) * o(t[0], t[1], s[1]) < 0.0 { x += 1; }
            }}
        }}
        assert_eq!(x, 0);
    }

    #[test]
    fn cena_e_render() {
        let letters: Vec<ScreenLetter> = extract_letters(1280, 720, 2, 128, 8, fake_alpha).into_iter().map(ScreenLetter::from_edge).collect();
        let nulls = [V2::new(150.0, 120.0), V2::new(1150.0, 150.0), V2::new(640.0, 650.0), V2::new(200.0, 600.0)];
        for nc in [false, true] {
            let p = SceneParams { curve: CurveParams { no_cross: nc, comp_half: 640.0, ..Default::default() }, progress: 0.8, nn_on: true, lines_min: 4, lines_max: 7, ..Default::default() };
            let items = build_scene(&letters, &nulls, &p);
            assert!(items.len() > 40);
            let mut cv = Canvas::new(1280, 720);
            render_scene(&items, &p, &|v| v, 1.0, &mut cv);
            // grava PPM para inspecao
            let mut buf = format!("P6 1280 720 255\n").into_bytes();
            for i in 0..1280 * 720 {
                let a = cv.rgba[i * 4 + 3];
                for k in 0..3 { buf.push(((cv.rgba[i * 4 + k] + 0.06 * (1.0 - a)) * 255.0).min(255.0) as u8); }
            }
            std::fs::write(format!("{}/test_{}.ppm", std::env::temp_dir().display(), if nc { "nc" } else { "arc" }), buf).unwrap();
        }
    }

    #[test]
    fn camera_padrao_centro() {
        let cam = Camera::default_for(1920.0, 1080.0, 2666.7);
        let p = cam.project(V3 { x: 960.0, y: 540.0, z: 0.0 });
        assert!((p.x - 960.0).abs() < 1e-6 && (p.y - 540.0).abs() < 1e-6);
        let q = cam.project(V3 { x: 1960.0, y: 540.0, z: 2666.7 }); // 2x mais longe -> metade do deslocamento
        assert!((q.x - 1460.0).abs() < 1e-6);
    }
}
