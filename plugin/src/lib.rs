//! Linhas Conectoras - efeito do After Effects.
//!
//! Aplique num solido 2D do tamanho da comp. Escolha o layer de origem
//! (texto/solido: o ALPHA vira o contorno) e ate 20 nulls. As linhas sao
//! desenhadas em screen space (seguem camera e layers 3D).

use after_effects as ae;
use ae::aegp;
use lc_core::*;
use std::sync::atomic::{AtomicI32, Ordering};

static AEGP_PLUGIN_ID: AtomicI32 = AtomicI32::new(0);

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
    Source,
    AlphaThresh,
    MinLetter,

    NullsStart,
    Null1, Null2, Null3, Null4, Null5, Null6, Null7, Null8, Null9, Null10,
    Null11, Null12, Null13, Null14, Null15, Null16, Null17, Null18, Null19, Null20,
    NullsEnd,

    DistStart,
    LinesMin,
    LinesMax,
    LettersPerNull,
    DistEnd,

    TrajStart,
    Curv,
    CurvVar,
    ArcUp,
    Invert,
    NoCross,
    Dispersion,
    TrajEnd,

    AnimStart,
    Progress,
    TimeRand,
    Seed,
    AnimEnd,

    StyleStart,
    Color,
    HueVar,
    BriVar,
    Width,
    WidthVar,
    Opacity,
    StyleEnd,

    MarkStart,
    MarkShape,
    MarkSize,
    MarkSizeVar,
    MarkWidth,
    MarkWithLine,
    MarkEnd,

    NnStart,
    NnOn,
    NnNeighbors,
    NnPerPair,
    NnEnd,

    OutStart,
    KeepOriginal,
    Debug,
    OutEnd,
}

const NULLS: [Params; 20] = [
    Params::Null1, Params::Null2, Params::Null3, Params::Null4, Params::Null5,
    Params::Null6, Params::Null7, Params::Null8, Params::Null9, Params::Null10,
    Params::Null11, Params::Null12, Params::Null13, Params::Null14, Params::Null15,
    Params::Null16, Params::Null17, Params::Null18, Params::Null19, Params::Null20,
];

#[derive(Default)]
struct Plugin {}

ae::define_effect!(Plugin, (), Params);

// ---------------------------------------------------------------- params
fn fslider(min: f32, max: f32, smin: f32, smax: f32, dflt: f64, percent: bool) -> ae::FloatSliderDef<'static> {
    ae::FloatSliderDef::setup(|f| {
        f.set_valid_min(min);
        f.set_valid_max(max);
        f.set_slider_min(smin);
        f.set_slider_max(smax);
        f.set_default(dflt);
        f.set_value(dflt);
        f.set_precision(1);
        if percent {
            f.set_display_flags(ae::ValueDisplayFlag::PERCENT);
        }
    })
}

fn islider(min: i32, max: i32, smin: i32, smax: i32, dflt: i32) -> ae::SliderDef<'static> {
    ae::SliderDef::setup(|f| {
        f.set_valid_min(min);
        f.set_valid_max(max);
        f.set_slider_min(smin);
        f.set_slider_max(smax);
        f.set_default(dflt);
        f.set_value(dflt);
    })
}

fn checkbox(dflt: bool) -> ae::CheckBoxDef<'static> {
    ae::CheckBoxDef::setup(|f| {
        f.set_default(dflt);
        f.set_value(dflt);
    })
}

impl AdobePluginGlobal for Plugin {
    fn params_setup(&self, params: &mut ae::Parameters<Params>, _: ae::InData, _: ae::OutData) -> Result<(), Error> {
        params.add(Params::Source, "Layer de origem (alpha)", ae::LayerDef::new())?;
        params.add(Params::AlphaThresh, "Limiar do alpha", islider(1, 255, 1, 255, 128))?;
        params.add(Params::MinLetter, "Tamanho minimo da letra", islider(1, 2000, 1, 200, 12))?;

        params.add_group(Params::NullsStart, Params::NullsEnd, "Nulls", false, |params| {
            for (i, p) in NULLS.iter().enumerate() {
                params.add(*p, &format!("Null {}", i + 1), ae::LayerDef::new())?;
            }
            Ok(())
        })?;

        params.add_group(Params::DistStart, Params::DistEnd, "Distribuicao", false, |params| {
            params.add(Params::LinesMin, "Linhas por letra (min)", islider(1, 100, 1, 20, 3))?;
            params.add(Params::LinesMax, "Linhas por letra (max)", islider(1, 100, 1, 20, 6))?;
            params.add(Params::LettersPerNull, "Letras por null (min)", islider(0, 100, 0, 20, 3))?;
            Ok(())
        })?;

        params.add_group(Params::TrajStart, Params::TrajEnd, "Trajetoria", false, |params| {
            params.add(Params::Curv, "Curvatura", fslider(-200.0, 200.0, 0.0, 100.0, 30.0, true))?;
            params.add(Params::CurvVar, "Variacao da curvatura", fslider(0.0, 100.0, 0.0, 100.0, 40.0, true))?;
            params.add(Params::ArcUp, "Arco para cima", checkbox(true))?;
            params.add(Params::Invert, "Inverter", checkbox(false))?;
            params.add(Params::NoCross, "Sem cruzamento", checkbox(true))?;
            params.add(Params::Dispersion, "Dispersao no null (px)", fslider(0.0, 1000.0, 0.0, 100.0, 10.0, false))?;
            Ok(())
        })?;

        params.add_group(Params::AnimStart, Params::AnimEnd, "Animacao", false, |params| {
            params.add(Params::Progress, "Progresso", fslider(0.0, 100.0, 0.0, 100.0, 100.0, true))?;
            params.add(Params::TimeRand, "Aleatoriedade do tempo", fslider(0.0, 95.0, 0.0, 95.0, 50.0, true))?;
            params.add(Params::Seed, "Seed", islider(0, 100_000, 0, 1000, 1))?;
            Ok(())
        })?;

        params.add_group(Params::StyleStart, Params::StyleEnd, "Contorno", false, |params| {
            params.add(Params::Color, "Cor", ae::ColorDef::setup(|f| {
                let c = ae::Pixel8 { alpha: 255, red: 89, green: 204, blue: 255 };
                f.set_default(c);
                f.set_value(c);
            }))?;
            params.add(Params::HueVar, "Variacao de matiz (graus)", fslider(0.0, 180.0, 0.0, 180.0, 20.0, false))?;
            params.add(Params::BriVar, "Variacao de brilho", fslider(0.0, 100.0, 0.0, 100.0, 25.0, true))?;
            params.add(Params::Width, "Espessura", fslider(0.0, 100.0, 0.0, 20.0, 2.0, false))?;
            params.add(Params::WidthVar, "Variacao de espessura", fslider(0.0, 100.0, 0.0, 100.0, 40.0, true))?;
            params.add(Params::Opacity, "Opacidade", fslider(0.0, 100.0, 0.0, 100.0, 100.0, true))?;
            Ok(())
        })?;

        params.add_group(Params::MarkStart, Params::MarkEnd, "Marcador no null", false, |params| {
            params.add(Params::MarkShape, "Forma", ae::PopupDef::setup(|f| {
                f.set_options(&["Nenhum", "Circulo", "Circulo cheio", "Quadrado", "Cruz", "Losango"]);
                f.set_default(2);
                f.set_value(2);
            }))?;
            params.add(Params::MarkSize, "Tamanho (px)", fslider(0.0, 500.0, 0.0, 50.0, 8.0, false))?;
            params.add(Params::MarkSizeVar, "Variacao de tamanho", fslider(0.0, 100.0, 0.0, 100.0, 40.0, true))?;
            params.add(Params::MarkWidth, "Espessura do marcador", fslider(0.0, 50.0, 0.0, 10.0, 1.5, false))?;
            params.add(Params::MarkWithLine, "Aparece quando a linha chega", checkbox(true))?;
            Ok(())
        })?;

        params.add_group(Params::NnStart, Params::NnEnd, "Conexoes entre nulls", true, |params| {
            params.add(Params::NnOn, "Conectar nulls entre si", checkbox(false))?;
            params.add(Params::NnNeighbors, "Vizinhos por null (0 = todos)", islider(0, 19, 0, 19, 2))?;
            params.add(Params::NnPerPair, "Linhas por par", islider(1, 20, 1, 10, 1))?;
            Ok(())
        })?;

        params.add_group(Params::OutStart, Params::OutEnd, "Saida", true, |params| {
            params.add(Params::KeepOriginal, "Manter o layer original", checkbox(false))?;
            params.add(Params::Debug, "Debug (nulls e contorno)", checkbox(false))?;
            Ok(())
        })?;

        Ok(())
    }

    fn handle_command(&mut self, cmd: ae::Command, in_data: ae::InData, mut out_data: ae::OutData, params: &mut ae::Parameters<Params>) -> Result<(), ae::Error> {
        match cmd {
            ae::Command::About => {
                out_data.set_return_msg("Linhas Conectoras v0.1\rLinhas curvas do contorno (alpha) de um layer ate ate 20 nulls, em screen space.");
            }
            ae::Command::GlobalSetup => {
                if let Ok(suite) = aegp::suites::Utility::new() {
                    if let Ok(id) = suite.register_with_aegp("JMB LinhasConectoras") {
                        AEGP_PLUGIN_ID.store(id as i32, Ordering::Relaxed);
                    }
                }
            }
            ae::Command::Render { in_layer, mut out_layer } => {
                render(&in_data, params, &in_layer, &mut out_layer)?;
            }
            _ => {}
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- helpers
fn plugin_id() -> aegp::PluginId { AEGP_PLUGIN_ID.load(Ordering::Relaxed) as _ }

fn layer_xf(layer: &aegp::Layer, t: ae::Time) -> Option<LayerXf> {
    let m = layer.to_world_xform(t).ok()?;
    let is_3d = layer.is_3d().unwrap_or(false);
    Some(LayerXf { to_world: M4::from_slice(m.as_slice()), is_3d })
}

fn stream_xyz(v: &aegp::StreamValue) -> Option<(f64, f64, f64)> {
    match *v {
        aegp::StreamValue::ThreeD { x, y, z } | aegp::StreamValue::ThreeDSpatial { x, y, z } => Some((x, y, z)),
        aegp::StreamValue::TwoD { x, y } | aegp::StreamValue::TwoDSpatial { x, y } => Some((x, y, 0.0)),
        _ => None,
    }
}

/// Layer referenciado por um parametro do tipo layer (lido pelo stream do efeito).
fn layer_from_param(effect: &aegp::Effect, comp: &aegp::Composition, index: usize, t: ae::Time) -> Option<aegp::Layer> {
    let pid = plugin_id();
    let stream = effect.new_stream_by_index(pid, index as i32).ok()?;
    let value = stream.new_value(pid, aegp::TimeMode::CompTime, t, true).ok()?;
    let aegp::StreamValue::LayerId(id) = value else { return None };
    if id == 0 { return None; }
    let handle = aegp::suites::Layer::new().ok()?.layer_from_layer_id(comp.handle(), id as _).ok()?;
    Some(aegp::Layer::from(handle))
}

fn clear_world(out: &mut ae::Layer) {
    let (w, h) = (out.width(), out.height());
    match out.bit_depth() {
        16 => for y in 0..h { for x in 0..w { *out.as_pixel16_mut(x, y) = ae::Pixel16 { alpha: 0, red: 0, green: 0, blue: 0 }; } },
        32 => for y in 0..h { for x in 0..w { *out.as_pixel32_mut(x, y) = ae::PixelF32 { alpha: 0.0, red: 0.0, green: 0.0, blue: 0.0 }; } },
        _ => for y in 0..h { for x in 0..w { *out.as_pixel8_mut(x, y) = ae::Pixel8 { alpha: 0, red: 0, green: 0, blue: 0 }; } },
    }
}

/// Compoe o Canvas (float premultiplicado) por cima do buffer de saida.
fn composite(cv: &Canvas, out: &mut ae::Layer) {
    let (w, h) = (out.width().min(cv.w), out.height().min(cv.h));
    let depth = out.bit_depth();
    for y in 0..h {
        for x in 0..w {
            let i = (y * cv.w + x) * 4;
            let s = &cv.rgba[i..i + 4];
            let a = s[3];
            if a <= 0.0 { continue; }
            let inv = 1.0 - a;
            match depth {
                16 => {
                    let p = out.as_pixel16_mut(x, y);
                    let m = ae::MAX_CHANNEL16 as f32;
                    p.red = (s[0] * m + p.red as f32 * inv + 0.5).min(m) as u16;
                    p.green = (s[1] * m + p.green as f32 * inv + 0.5).min(m) as u16;
                    p.blue = (s[2] * m + p.blue as f32 * inv + 0.5).min(m) as u16;
                    p.alpha = (a * m + p.alpha as f32 * inv + 0.5).min(m) as u16;
                }
                32 => {
                    let p = out.as_pixel32_mut(x, y);
                    p.red = s[0] + p.red * inv;
                    p.green = s[1] + p.green * inv;
                    p.blue = s[2] + p.blue * inv;
                    p.alpha = a + p.alpha * inv;
                }
                _ => {
                    let p = out.as_pixel8_mut(x, y);
                    p.red = (s[0] * 255.0 + p.red as f32 * inv + 0.5).min(255.0) as u8;
                    p.green = (s[1] * 255.0 + p.green as f32 * inv + 0.5).min(255.0) as u8;
                    p.blue = (s[2] * 255.0 + p.blue as f32 * inv + 0.5).min(255.0) as u8;
                    p.alpha = (a * 255.0 + p.alpha as f32 * inv + 0.5).min(255.0) as u8;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- render
fn render(in_data: &ae::InData, params: &mut ae::Parameters<Params>, in_layer: &ae::Layer, out: &mut ae::Layer) -> Result<(), ae::Error> {
    let dsx = f64::from(in_data.downsample_x());
    let dsy = f64::from(in_data.downsample_y());

    // saida base
    if params.get(Params::KeepOriginal)?.as_checkbox()?.value() {
        out.copy_from(in_layer, None, None)?;
    } else {
        clear_world(out);
    }

    // tempo da comp, layer do efeito, comp
    let effect = in_data.effect();
    let t = effect.comp_time(in_data.current_time(), in_data.time_scale())?;
    let my_layer = effect.layer()?;
    let comp = my_layer.parent_comp()?;
    let (cw, ch) = comp.item()?.dimensions()?;
    let (cw, ch) = (cw as f64, ch as f64);

    // camera ativa (ou a padrao do AE)
    let mut cam = None;
    if let Ok(Some(_cam_layer)) = effect.camera(t) {
        if let Ok((m, dist, _, _)) = effect.camera_matrix(t) {
            if dist > 0.0 {
                if let Some(inv) = M4::from_slice(m.as_slice()).invert() {
                    cam = Some(Camera { world_to_cam: inv, zoom: dist, cx: cw * 0.5, cy: ch * 0.5 });
                }
            }
        }
    }
    let cam = cam.unwrap_or_else(|| {
        let d = aegp::suites::Camera::new()
            .and_then(|s| s.default_camera_distance_to_image_plane(comp.handle()))
            .unwrap_or(0.0);
        let d = if d > 0.0 { d } else { (cw * 0.5) / (39.5978f64.to_radians() * 0.5).tan() };
        Camera::default_for(cw, ch, d)
    });

    // comp -> buffer de saida (inversa do layer do efeito)
    let comp_to_layer = layer_xf(&my_layer, t).and_then(|x| x.to_world.invert());
    let to_buf = move |p: V2| -> V2 {
        let l = match &comp_to_layer {
            Some(m) => m.xform(V3 { x: p.x, y: p.y, z: 0.0 }),
            None => V3 { x: p.x, y: p.y, z: 0.0 },
        };
        V2::new(l.x * dsx, l.y * dsy)
    };

    // layers referenciados pelos parametros
    let aegp_effect = aegp::Effect::new(in_data.effect_ref(), plugin_id())?;

    let mut nulls: Vec<V2> = Vec::new();
    for p in NULLS {
        let Some(idx) = params.index(p) else { continue };
        let Some(nl) = layer_from_param(&aegp_effect, &comp, idx, t) else { continue };
        let Some(xf) = layer_xf(&nl, t) else { continue };
        let (ax, ay, az) = nl
            .layer_stream_value(aegp::LayerStream::AnchorPoint, aegp::TimeMode::CompTime, t, false)
            .ok()
            .and_then(|v| stream_xyz(&v))
            .unwrap_or((0.0, 0.0, 0.0));
        nulls.push(xf.to_screen(&cam, ax, ay, az));
    }

    // contorno do layer de origem
    let mut letters: Vec<ScreenLetter> = Vec::new();
    let src_layer = params.index(Params::Source).and_then(|idx| layer_from_param(&aegp_effect, &comp, idx, t));
    if let Some(src_layer) = src_layer {
        if let Some(sxf) = layer_xf(&src_layer, t) {
            let checkout = params.checkout(Params::Source)?;
            let def = checkout.as_layer()?;
            if let Some(sw) = def.value() {
                let (w, h) = (sw.width(), sw.height());
                let depth = sw.bit_depth();
                let stride = ((w.max(h) as f64 / 1400.0).ceil() as usize).max(1);
                let thresh = params.get(Params::AlphaThresh)?.as_slider()?.value().clamp(1, 255) as u8;
                let min_edge = (params.get(Params::MinLetter)?.as_slider()?.value().max(1) as usize / stride).max(1);
                let raw = extract_letters(w, h, stride, thresh, min_edge, |x, y| match depth {
                    16 => (sw.as_pixel16(x, y).alpha as u32 * 255 / ae::MAX_CHANNEL16 as u32) as u8,
                    32 => (sw.as_pixel32(x, y).alpha.clamp(0.0, 1.0) * 255.0) as u8,
                    _ => sw.as_pixel8(x, y).alpha,
                });
                // pixel do buffer (reduzido pela resolucao) -> espaco do layer -> tela
                let o = sw.origin();
                let (ox, oy) = (o.h as f64, o.v as f64);
                for edge in raw {
                    let pts: Vec<V2> = edge.iter().map(|p| sxf.to_screen(&cam, p.x / dsx + ox, p.y / dsy + oy, 0.0)).collect();
                    letters.push(ScreenLetter::from_edge(pts));
                }
            }
        }
    }

    // parametros da cena
    let c = params.get(Params::Color)?.as_color()?.value();
    let lines_min = params.get(Params::LinesMin)?.as_slider()?.value();
    let p = SceneParams {
        lines_min,
        lines_max: params.get(Params::LinesMax)?.as_slider()?.value().max(lines_min),
        letters_per_null: params.get(Params::LettersPerNull)?.as_slider()?.value(),
        curve: CurveParams {
            curv: params.get(Params::Curv)?.as_float_slider()?.value() / 100.0,
            vari: params.get(Params::CurvVar)?.as_float_slider()?.value() / 100.0,
            up: params.get(Params::ArcUp)?.as_checkbox()?.value(),
            inv: params.get(Params::Invert)?.as_checkbox()?.value(),
            no_cross: params.get(Params::NoCross)?.as_checkbox()?.value(),
            comp_half: cw.max(ch) * 0.5,
        },
        disp: params.get(Params::Dispersion)?.as_float_slider()?.value(),
        progress: params.get(Params::Progress)?.as_float_slider()?.value() / 100.0,
        time_rand: params.get(Params::TimeRand)?.as_float_slider()?.value() / 100.0,
        seed: params.get(Params::Seed)?.as_slider()?.value().max(0) as u64,
        color: Rgb { r: c.red as f64 / 255.0, g: c.green as f64 / 255.0, b: c.blue as f64 / 255.0 },
        opacity: params.get(Params::Opacity)?.as_float_slider()?.value() / 100.0,
        hue_var: params.get(Params::HueVar)?.as_float_slider()?.value(),
        bri_var: params.get(Params::BriVar)?.as_float_slider()?.value() / 100.0,
        width: params.get(Params::Width)?.as_float_slider()?.value(),
        width_var: params.get(Params::WidthVar)?.as_float_slider()?.value() / 100.0,
        mark_shape: params.get(Params::MarkShape)?.as_popup()?.value() - 1, // popup e 1-based
        mark_size: params.get(Params::MarkSize)?.as_float_slider()?.value(),
        mark_size_var: params.get(Params::MarkSizeVar)?.as_float_slider()?.value() / 100.0,
        mark_width: params.get(Params::MarkWidth)?.as_float_slider()?.value(),
        mark_with_line: params.get(Params::MarkWithLine)?.as_checkbox()?.value(),
        nn_on: params.get(Params::NnOn)?.as_checkbox()?.value(),
        nn_neighbors: params.get(Params::NnNeighbors)?.as_slider()?.value(),
        nn_per_pair: params.get(Params::NnPerPair)?.as_slider()?.value(),
    };

    let items = build_scene(&letters, &nulls, &p);

    // desenha
    let mut cv = Canvas::new(out.width(), out.height());
    let scale = 0.5 * (dsx + dsy);
    render_scene(&items, &p, &to_buf, scale, &mut cv);

    if params.get(Params::Debug)?.as_checkbox()?.value() {
        cv.begin(Rgb { r: 1.0, g: 0.2, b: 0.2 }, 1.0);
        for n in &nulls { cv.marker(4, to_buf(*n), 30.0 * scale, 2.0 * scale); }
        cv.begin(Rgb { r: 1.0, g: 1.0, b: 0.2 }, 1.0);
        for l in &letters {
            for q in l.edge.iter().step_by(4) {
                let b = to_buf(*q);
                cv.plot(b.x as i64, b.y as i64, 1.0);
            }
        }
    }

    composite(&cv, out);
    Ok(())
}
