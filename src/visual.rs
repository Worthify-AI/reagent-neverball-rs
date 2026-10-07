// SPDX-License-Identifier: GPL-3.0-or-later
//! Presentation reconstructed from licensed compiled assets and runtime images.
//! No reference positions enter the simulation; all transforms use Game state.
use macroquad::prelude::*;
use reagent_neverball_rs::Game;
use serde::Deserialize;
#[derive(Deserialize)]
struct Triangle {
    m: usize,
    p: [[f32; 3]; 3],
    n: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
}
#[derive(Deserialize)]
struct Visual {
    triangles: Vec<Triangle>,
}
#[derive(Deserialize)]
struct Billboard {
    m: usize,
    distance: f32,
    width: f32,
    height: f32,
    rx: f32,
    ry: f32,
    rz: f32,
}
struct Part {
    mesh: Mesh,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
}
pub struct Scene {
    course: Vec<Part>,
    ball: Vec<Part>,
    coin: Vec<Part>,
    background: Vec<Mesh>,
    sky: Texture2D,
    material: Material,
    backdrop_material: Material,
}
fn texture(data: &[u8]) -> Texture2D {
    let t = Texture2D::from_file_with_format(data, None);
    t.set_filter(FilterMode::Linear);
    t
}
macro_rules! tex {
    ($p:literal) => {
        texture(include_bytes!(concat!("../assets/neverball/", $p)))
    };
}
fn parts(data: &str, textures: &[Option<Texture2D>]) -> Vec<Part> {
    let v: Visual = serde_json::from_str(data).expect("licensed visual fixture");
    let mut result = Vec::new();
    for (mi, texture) in textures.iter().enumerate() {
        let Some(texture) = texture else { continue };
        let ts: Vec<_> = v.triangles.iter().filter(|t| t.m == mi).collect();
        for chunk in ts.chunks(180) {
            let mut vertices = Vec::new();
            let mut positions = Vec::new();
            let mut normals = Vec::new();
            for t in chunk {
                for i in 0..3 {
                    let p = Vec3::from_array(t.p[i]);
                    let n = Vec3::from_array(t.n[i]);
                    let mut vert = Vertex::new2(p, vec2(t.uv[i][0], 1. - t.uv[i][1]), WHITE);
                    vert.normal = n.extend(0.);
                    vertices.push(vert);
                    positions.push(p);
                    normals.push(n);
                }
            }
            result.push(Part {
                mesh: Mesh {
                    indices: (0..vertices.len() as u16).collect(),
                    vertices,
                    texture: Some(texture.clone()),
                },
                positions,
                normals,
            });
        }
    }
    result
}
const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
uniform mat4 Model;
uniform mat4 Projection;
varying lowp vec4 color;
varying mediump vec2 uv;
varying highp vec3 pos;
varying highp vec3 norm;
void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;color=color0/255.0;pos=position;norm=normal.xyz;}
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying mediump vec2 uv;
varying highp vec3 pos;
varying highp vec3 norm;
uniform sampler2D Texture;
uniform vec3 Eye;
uniform float Gloss;
void main(){
 vec4 t=texture2D(Texture,fract(uv));
 vec3 n=normalize(norm);vec3 l=normalize(vec3(-0.3,1.0,0.4));
 float diffuse=0.48+0.52*max(dot(n,l),0.0);
 vec3 h=normalize(l+normalize(Eye-pos));
 float spec=Gloss*pow(max(dot(n,h),0.0),28.0);
 gl_FragColor=vec4(t.rgb*diffuse+vec3(spec),t.a)*color;
 if(gl_FragColor.a<0.01)discard;
}
"#;
impl Scene {
    pub fn new() -> Self {
        let course = parts(
            include_str!("../fixtures/course-visual.json"),
            &[
                None,
                Some(tex!("textures/mtrl/turf-grey.png")),
                Some(tex!("textures/mtrl/turf-green-small.png")),
                Some(tex!("textures/mtrl/edge-green-offset.png")),
                Some(tex!("textures/mtrl/turf-green.png")),
                Some(tex!("textures/mtrl/coin-green-small.png")),
                Some(tex!("textures/mtrl/turf-green-offset.png")),
                Some(tex!("textures/mtrl/turf-green-dark.png")),
                Some(tex!("textures/mtrl/chrome.png")),
                Some(tex!("textures/mtrl/arrow-green-light.png")),
                Some(tex!("textures/mtrl/goal.png")),
            ],
        );
        let ball = parts(
            include_str!("../fixtures/ball-visual.json"),
            &[Some(tex!("ball/basic-ball/basic-ball.png"))],
        );
        let coin = parts(
            include_str!("../fixtures/coin-visual.json"),
            &[Some(tex!("item/coin/coin.png"))],
        );
        let textures = [
            tex!("png/contrail.png"),
            tex!("png/clouds1.png"),
            tex!("png/clouds3.png"),
            tex!("png/clouds2.png"),
            tex!("png/hills1.png"),
            tex!("png/hills3.png"),
            tex!("png/hills2.png"),
        ];
        let bills: Vec<Billboard> =
            serde_json::from_str(include_str!("../fixtures/background-visual.json")).unwrap();
        let background = bills
            .iter()
            .filter(|b| b.m != 0)
            .map(|b| {
                let q = Quat::from_rotation_y(b.ry.to_radians())
                    * Quat::from_rotation_x(b.rx.to_radians())
                    * Quat::from_rotation_z(b.rz.to_radians());
                let vertices = [
                    (-0.5, 1.0, 0., 0.),
                    (0.5, 1.0, 1., 0.),
                    (0.5, 0.0, 1., 1.),
                    (-0.5, 0.0, 0., 1.),
                ]
                .iter()
                .map(|&(x, y, u, v)| {
                    Vertex::new2(
                        q * vec3(x * b.width, y * b.height, -b.distance),
                        vec2(u, v),
                        WHITE,
                    )
                })
                .collect();
                Mesh {
                    vertices,
                    indices: vec![0, 1, 2, 0, 2, 3],
                    texture: Some(textures[b.m].clone()),
                }
            })
            .collect();
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                pipeline_params: miniquad::PipelineParams {
                    depth_test: miniquad::Comparison::LessOrEqual,
                    depth_write: true,
                    color_blend: Some(miniquad::BlendState::new(
                        miniquad::Equation::Add,
                        miniquad::BlendFactor::Value(miniquad::BlendValue::SourceAlpha),
                        miniquad::BlendFactor::OneMinusValue(miniquad::BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                uniforms: vec![
                    UniformDesc::new("Eye", UniformType::Float3),
                    UniformDesc::new("Gloss", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .expect("presentation shader");
        let backdrop_material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying mediump vec2 uv;
uniform sampler2D Texture;
void main(){gl_FragColor=texture2D(Texture,uv)*color;}
"#,
            },
            MaterialParams {
                pipeline_params: miniquad::PipelineParams {
                    depth_test: miniquad::Comparison::Always,
                    depth_write: false,
                    color_blend: Some(miniquad::BlendState::new(
                        miniquad::Equation::Add,
                        miniquad::BlendFactor::Value(miniquad::BlendValue::SourceAlpha),
                        miniquad::BlendFactor::OneMinusValue(miniquad::BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("backdrop shader");
        Self {
            course,
            ball,
            coin,
            background,
            sky: tex!("back/land.png"),
            material,
            backdrop_material,
        }
    }
    pub fn draw(&mut self, g: &Game, yaw: f32, wide: bool) {
        clear_background(SKYBLUE);
        // The shipped gradient is a latitude strip. Show the horizon portion at
        // the observed camera pitch rather than stretching the whole night sky.
        draw_texture_ex(
            &self.sky,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                source: Some(Rect::new(0., 56., 16., 34.)),
                ..Default::default()
            },
        );
        let dist = if wide { 5.0 } else { 2.0 };
        let height = if wide { 3.5 } else { 0.75 };
        let eye = g.position + vec3(yaw.sin() * dist, height, yaw.cos() * dist);
        let target = g.position + vec3(0., 0.25, 0.);
        set_camera(&Camera3D {
            position: Vec3::ZERO,
            target: target - eye,
            up: Vec3::Y,
            fovy: 50f32.to_radians(),
            z_near: 0.05,
            z_far: 500.,
            ..Default::default()
        });
        gl_use_material(&self.backdrop_material);
        for m in &self.background {
            draw_mesh(m);
        }
        gl_use_default_material();
        set_camera(&Camera3D {
            position: eye,
            target,
            up: Vec3::Y,
            fovy: 50f32.to_radians(),
            z_near: 0.05,
            z_far: 500.,
            ..Default::default()
        });
        self.material.set_uniform("Eye", eye);
        self.material.set_uniform("Gloss", 0.0f32);
        gl_use_material(&self.material);
        let tilt = Quat::from_rotation_x(-g.tilt.y.to_radians())
            * Quat::from_rotation_z(-g.tilt.x.to_radians());
        for p in &mut self.course {
            draw_part(p, tilt, g.position - tilt * g.position, 1.);
        }
        self.material.set_uniform("Gloss", 0.65f32);
        let turn = Quat::from_rotation_y((g.course.time_limit - g.time) * 3.);
        for (i, p) in g.course.coins.iter().enumerate() {
            if !g.collected[i] {
                let pos = g.position + tilt * (Vec3::from_array(*p) - g.position);
                for m in &mut self.coin {
                    for v in &mut m.mesh.vertices {
                        v.color = [255, 220, 0, 255];
                    }
                    draw_part(m, tilt * turn, pos, 0.1);
                }
            }
        }
        gl_use_default_material();
        // Small contact shadow is driven solely by the simulated ball position.
        let y = 0.004;
        let center = g.position + tilt * (vec3(g.position.x, y, g.position.z) - g.position);
        for i in 0..32 {
            let a = i as f32 * std::f32::consts::TAU / 32.;
            let b = (i + 1) as f32 * std::f32::consts::TAU / 32.;
            let r = g.course.radius * 0.94;
            let c = Color::new(0., 0., 0., 0.25);
            draw_mesh(&Mesh {
                vertices: vec![
                    Vertex::new2(center, Vec2::ZERO, c),
                    Vertex::new2(
                        center + tilt * vec3(a.cos() * r, 0., a.sin() * r),
                        Vec2::ZERO,
                        c,
                    ),
                    Vertex::new2(
                        center + tilt * vec3(b.cos() * r, 0., b.sin() * r),
                        Vec2::ZERO,
                        c,
                    ),
                ],
                indices: vec![0, 1, 2],
                texture: None,
            });
        }
        gl_use_material(&self.material);
        self.material.set_uniform("Gloss", 1.5f32);
        let rolling = Quat::from_rotation_x(g.distance / g.course.radius);
        for m in &mut self.ball {
            draw_part(m, rolling, g.position, g.course.radius);
        }
        gl_use_default_material();
        if g.goal_open() {
            let center = g.position
                + tilt
                    * (Vec3::from_array([g.course.goal[0], 0.01, g.course.goal[2]]) - g.position);
            draw_cylinder(
                center + tilt * Vec3::Y * 0.5,
                g.course.goal[3],
                g.course.goal[3],
                1.,
                None,
                Color::new(0.2, 1., 0.3, 0.2),
            );
        }
        set_default_camera();
    }
}
fn draw_part(p: &mut Part, q: Quat, offset: Vec3, scale: f32) {
    for ((v, base), n) in p.mesh.vertices.iter_mut().zip(&p.positions).zip(&p.normals) {
        v.position = offset + q * (*base * scale);
        v.normal = (q * *n).extend(0.);
    }
    draw_mesh(&p.mesh);
}
