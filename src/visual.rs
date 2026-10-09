// SPDX-License-Identifier: GPL-3.0-or-later
//! Presentation reconstructed from licensed compiled assets and runtime images.
//! No reference positions enter the simulation; all transforms use Game state.
use macroquad::prelude::*;
use reagent_neverball_rs::{
    entities::{FullGame, PathRuntime},
    sol::Sol,
};
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
    billboard: Option<reagent_neverball_rs::sol::Billboard>,
    flags: i32,
    shine: f32,
    ambient: Vec3,
    specular: Vec3,
    emission: Vec3,
    alpha_clip: f32,
    body: usize,
    mesh: Mesh,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
}
struct Model {
    parts: Vec<Part>,
    sol: Sol,
    paths: PathRuntime,
    elapsed: f32,
}
impl From<Vec<Part>> for Model {
    fn from(parts: Vec<Part>) -> Self {
        let sol = Sol::default();
        let paths = PathRuntime::new(&sol);
        Self {
            parts,
            sol,
            paths,
            elapsed: 0.,
        }
    }
}
impl Model {
    fn at(&mut self, time: f32) -> Vec<reagent_neverball_rs::physics::BodyPose> {
        if time < self.elapsed {
            self.paths = PathRuntime::new(&self.sol);
            self.elapsed = 0.;
        }
        self.paths.advance(&self.sol, (time - self.elapsed).max(0.));
        self.elapsed = time;
        self.paths.poses(&self.sol, 0.)
    }
}
pub struct Scene {
    pub background_enabled: bool,
    pub textures_enabled: bool,
    pub shadow_enabled: bool,
    pub reflection_enabled: bool,
    pub fov: f32,
    passes: Vec<Material>,
    grow: Model,
    shrink: Model,
    coin5: Model,
    coin10: Model,
    goal: Model,
    jump: Model,
    shadow: Option<Texture2D>,
    beam: Model,
    course: Vec<Part>,
    ball: Model,
    ball_inner: Model,
    ball_outer: Model,
    pub ball_file: String,
    filtering: Option<bool>,
    coin: Model,
    background: Vec<Mesh>,
    background_data: Vec<reagent_neverball_rs::sol::Billboard>,
    sky_model: Model,
    pub animation_time: f32,
    pub goal_height: f32,
    sky: Texture2D,
    backdrop_material: Material,
    mirror_backdrop: Material,
    mirror_mask: Material,
}
fn polygon_offset(enabled: bool) {
    thread_local! { static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    ENABLED.with(|state| {
        if state.replace(enabled) != enabled {
            unsafe {
                get_internal_gl().flush();
                if enabled {
                    miniquad::gl::glEnable(0x8037);
                    miniquad::gl::glPolygonOffset(-1., -2.);
                } else {
                    miniquad::gl::glDisable(0x8037);
                }
            }
        }
    });
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
                billboard: None,
                flags: 0x800,
                shine: 20.,
                ambient: Vec3::splat(0.2),
                specular: Vec3::ZERO,
                emission: Vec3::ZERO,
                alpha_clip: -1.,
                body: 0,
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
uniform mediump float PointSprite;
uniform mediump float PointSize;
uniform mediump vec2 ScreenSize;
uniform mediump float MirrorEnabled;
uniform mediump vec3 MirrorNormal;
uniform mediump vec3 MirrorOrigin;
varying lowp vec4 color;
varying mediump vec2 uv;
varying highp vec3 pos;
varying highp vec3 norm;
varying highp vec3 unmirrored;
uniform mediump float GoalLight;
uniform mediump vec3 GoalDirection;
uniform mediump float Lighting;
uniform mediump float Gloss;
uniform mediump vec3 MaterialAmbient;
uniform mediump vec3 MaterialSpecular;
uniform mediump vec3 MaterialEmission;
uniform mediump vec3 CameraBack;
varying lowp vec3 litPrimary;
varying lowp vec3 litSecondary;
varying mediump vec2 sphereUV;
uniform mediump vec3 Eye;
uniform mediump vec3 CameraRight;
uniform mediump vec3 CameraUp;
void main(){vec3 worldPosition=position;vec3 worldNormal=normal.xyz;
if(MirrorEnabled>0.5){worldPosition-=2.0*MirrorNormal*dot(worldPosition-MirrorOrigin,MirrorNormal);worldNormal-=2.0*MirrorNormal*dot(worldNormal,MirrorNormal);}
gl_Position=Projection*Model*vec4(worldPosition,1.0);if(PointSprite>0.5){float pixels=clamp(PointSize/max(length(worldPosition-Eye),0.0001),1.0,ScreenSize.y*0.25);gl_Position.xy+=vec2(texcoord.x-0.5,0.5-texcoord.y)*pixels*2.0/ScreenSize*gl_Position.w;}uv=texcoord;color=color0/255.0;pos=worldPosition;norm=worldNormal;unmirrored=position;
 vec3 n=normalize(worldNormal);
 vec3 l0=normalize(vec3(-8.0,32.0,-8.0));vec3 l1=normalize(vec3(8.0,32.0,8.0));
 if(MirrorEnabled>0.5){l0-=2.0*MirrorNormal*dot(l0,MirrorNormal);l1-=2.0*MirrorNormal*dot(l1,MirrorNormal);}
 vec3 l2=normalize(GoalDirection);if(MirrorEnabled>0.5)l2-=2.0*MirrorNormal*dot(l2,MirrorNormal);
 float d2=max(dot(n,l2),0.0)*GoalLight;
 float d0=max(dot(n,l0),0.0)*(1.0-GoalLight);float d1=max(dot(n,l1),0.0)*(1.0-GoalLight);
 vec3 c0=vec3(1.0,0.8,0.8);vec3 c1=vec3(0.8,1.0,0.8);
 vec3 primary=(color0.rgb/255.0);vec3 secondary=vec3(0.0);
 if(Lighting>0.5){
  primary=clamp(MaterialEmission+MaterialAmbient*(1.6-1.4*GoalLight)+(color0.rgb/255.0)*(c0*d0+c1*d1+vec3(d2)),0.0,1.0);
  float s0=d0>0.0?pow(max(dot(n,normalize(l0+CameraBack)),0.0),Gloss):0.0;
  float s1=d1>0.0?pow(max(dot(n,normalize(l1+CameraBack)),0.0),Gloss):0.0;
  float s2=d2>0.0?pow(max(dot(n,normalize(l2+CameraBack)),0.0),Gloss)*GoalLight:0.0;
  secondary=MaterialSpecular*(c0*s0+c1*s1+vec3(s2));
 }
litPrimary=primary;litSecondary=secondary;
 vec3 environment=reflect(normalize(worldPosition-Eye),normalize(worldNormal));
 environment=vec3(dot(environment,CameraRight),dot(environment,CameraUp),dot(environment,CameraBack));
 float sphereDenominator=2.0*sqrt(environment.x*environment.x+environment.y*environment.y+(environment.z+1.0)*(environment.z+1.0));
 sphereUV=environment.xy/max(sphereDenominator,0.001)+0.5;
}
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec3 litPrimary;
varying lowp vec3 litSecondary;
varying mediump vec2 sphereUV;
varying lowp vec4 color;
varying mediump vec2 uv;
varying highp vec3 pos;
varying highp vec3 norm;
varying highp vec3 unmirrored;
uniform sampler2D Texture;
uniform sampler2D Shadow;
uniform float ShadowEnabled;
uniform float ShadowReceiver;
uniform vec3 ShadowCenter;
uniform vec3 ShadowX;
uniform vec3 ShadowY;
uniform vec3 ShadowZ;
uniform float ShadowScale;
uniform vec3 Eye;
uniform mediump float Gloss;
uniform mediump vec3 MaterialAmbient;
uniform mediump vec3 MaterialSpecular;
uniform mediump vec3 MaterialEmission;
uniform float Environment;
uniform mediump float Lighting;
uniform float AlphaClip;
uniform mediump float MirrorEnabled;
uniform mediump vec3 MirrorNormal;
uniform mediump vec3 MirrorOrigin;
uniform float ClipHalf;
uniform vec3 ClipCenter;
uniform vec3 ClipNormal;
uniform float TextureEnabled;
uniform float ReflectionEnabled;
uniform vec3 CameraRight;
uniform vec3 CameraUp;
uniform mediump vec3 CameraBack;
void main(){
 if(MirrorEnabled>0.5&&dot(pos-MirrorOrigin,MirrorNormal)>0.001)discard;
 if(ClipHalf*dot(pos-ClipCenter,ClipNormal)<0.0)discard;
 vec2 coordinate=Environment*ReflectionEnabled>0.5?sphereUV:fract(uv);
 vec4 t=TextureEnabled>0.5?texture2D(Texture,coordinate):vec4(1.0);
 vec3 shaded=t.rgb*litPrimary;
 if(ShadowEnabled*ShadowReceiver>0.5){
  vec3 delta=unmirrored-ShadowCenter;
  vec2 st=vec2(0.5+dot(delta,ShadowX)*ShadowScale,0.5-dot(delta,ShadowZ)*ShadowScale);
  float alpha=texture2D(Shadow,clamp(st,0.0,1.0)).a;
  float heightMask=clamp(0.5-2.0*dot(delta,ShadowY),0.0,1.0);
  shaded=t.rgb*litPrimary*(1.0-alpha*heightMask);
 }
 gl_FragColor=vec4(shaded+litSecondary,t.a*color.a);
 if(gl_FragColor.a<AlphaClip)discard;
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
        thread_local! {static MATERIAL_CACHE:std::cell::RefCell<Option<Vec<Material>>>=const{std::cell::RefCell::new(None)};}
        let passes = MATERIAL_CACHE.with(|cache| {
            if let Some(p) = cache.borrow().as_ref() {
                return p.clone();
            }
            let mut unique = std::collections::BTreeMap::new();
            let passes = (0..64)
                .map(|raw| {
                    let mirrored = raw >= 32;
                    let raw = raw & 31;
                    // These nine pipeline combinations cover every packaged SOL
                    // material and ball metadata combination, without exceeding
                    // Macroquad's32 pipeline slots when reflected.
                    let bits = match raw {
                        0..=5 | 8 | 9 | 21 => raw,
                        _ => {
                            if raw & 16 != 0 {
                                21
                            } else if raw & 8 != 0 {
                                8 | (raw & 1)
                            } else {
                                raw & 5
                            }
                        }
                    };
                    unique
                        .entry((bits, mirrored))
                        .or_insert_with(|| {
                            load_material(
                                ShaderSource::Glsl {
                                    vertex: VERTEX,
                                    fragment: FRAGMENT,
                                },
                                MaterialParams {
                                    pipeline_params: miniquad::PipelineParams {
                                        stencil_test: if mirrored {
                                            Some(stencil(false))
                                        } else {
                                            None
                                        },
                                        front_face_order: if mirrored {
                                            miniquad::FrontFaceOrder::Clockwise
                                        } else {
                                            miniquad::FrontFaceOrder::CounterClockwise
                                        },
                                        depth_test: if bits & 16 != 0 {
                                            miniquad::Comparison::Always
                                        } else {
                                            miniquad::Comparison::LessOrEqual
                                        },
                                        depth_write: bits & 1 == 0,
                                        cull_face: if bits & 8 != 0 {
                                            miniquad::CullFace::Front
                                        } else if bits & 2 != 0 {
                                            miniquad::CullFace::Nothing
                                        } else {
                                            miniquad::CullFace::Back
                                        },
                                        color_blend: Some(miniquad::BlendState::new(
                                            miniquad::Equation::Add,
                                            miniquad::BlendFactor::Value(
                                                miniquad::BlendValue::SourceAlpha,
                                            ),
                                            if bits & 4 != 0 {
                                                miniquad::BlendFactor::One
                                            } else {
                                                miniquad::BlendFactor::OneMinusValue(
                                                    miniquad::BlendValue::SourceAlpha,
                                                )
                                            },
                                        )),
                                        ..Default::default()
                                    },
                                    textures: vec!["Shadow".into()],
                                    uniforms: vec![
                                        UniformDesc::new("ShadowEnabled", UniformType::Float1),
                                        UniformDesc::new("ShadowReceiver", UniformType::Float1),
                                        UniformDesc::new("ShadowCenter", UniformType::Float3),
                                        UniformDesc::new("ShadowX", UniformType::Float3),
                                        UniformDesc::new("ShadowY", UniformType::Float3),
                                        UniformDesc::new("ShadowZ", UniformType::Float3),
                                        UniformDesc::new("ShadowScale", UniformType::Float1),
                                        UniformDesc::new("PointSprite", UniformType::Float1),
                                        UniformDesc::new("PointSize", UniformType::Float1),
                                        UniformDesc::new("ScreenSize", UniformType::Float2),
                                        UniformDesc::new("GoalLight", UniformType::Float1),
                                        UniformDesc::new("GoalDirection", UniformType::Float3),
                                        UniformDesc::new("MirrorEnabled", UniformType::Float1),
                                        UniformDesc::new("MirrorNormal", UniformType::Float3),
                                        UniformDesc::new("MirrorOrigin", UniformType::Float3),
                                        UniformDesc::new("ClipHalf", UniformType::Float1),
                                        UniformDesc::new("ClipCenter", UniformType::Float3),
                                        UniformDesc::new("ClipNormal", UniformType::Float3),
                                        UniformDesc::new("TextureEnabled", UniformType::Float1),
                                        UniformDesc::new("ReflectionEnabled", UniformType::Float1),
                                        UniformDesc::new("Eye", UniformType::Float3),
                                        UniformDesc::new("Gloss", UniformType::Float1),
                                        UniformDesc::new("MaterialAmbient", UniformType::Float3),
                                        UniformDesc::new("MaterialSpecular", UniformType::Float3),
                                        UniformDesc::new("MaterialEmission", UniformType::Float3),
                                        UniformDesc::new("Environment", UniformType::Float1),
                                        UniformDesc::new("Lighting", UniformType::Float1),
                                        UniformDesc::new("AlphaClip", UniformType::Float1),
                                        UniformDesc::new("CameraRight", UniformType::Float3),
                                        UniformDesc::new("CameraUp", UniformType::Float3),
                                        UniformDesc::new("CameraBack", UniformType::Float3),
                                    ],
                                },
                            )
                            .expect("SOL material pass")
                        })
                        .clone()
                })
                .collect::<Vec<_>>();
            passes
        });
        MATERIAL_CACHE.with(|cache| {
            if cache.borrow().is_none() {
                *cache.borrow_mut() = Some(passes.clone());
            }
        });
        let make_backdrop = |mirrored| {
            load_material(
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
                        stencil_test: if mirrored { Some(stencil(false)) } else { None },
                        depth_test: miniquad::Comparison::Always,
                        depth_write: false,
                        color_blend: Some(miniquad::BlendState::new(
                            miniquad::Equation::Add,
                            miniquad::BlendFactor::Value(miniquad::BlendValue::SourceAlpha),
                            miniquad::BlendFactor::OneMinusValue(miniquad::BlendValue::SourceAlpha),
                        )),
                        ..Default::default()
                    },
                    uniforms: vec![
                        UniformDesc::new("MirrorEnabled", UniformType::Float1),
                        UniformDesc::new("MirrorNormal", UniformType::Float3),
                        UniformDesc::new("MirrorOrigin", UniformType::Float3),
                    ],
                    ..Default::default()
                },
            )
            .expect("backdrop shader")
        };
        let backdrop_material = make_backdrop(false);
        let mirror_backdrop = make_backdrop(true);
        let mirror_mask = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment:
                    "#version 100\nprecision mediump float;void main(){gl_FragColor=vec4(1.0);}",
            },
            MaterialParams {
                pipeline_params: miniquad::PipelineParams {
                    depth_test: miniquad::Comparison::LessOrEqual,
                    depth_write: false,
                    color_write: (false, false, false, false),
                    stencil_test: Some(stencil(true)),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("reflection stencil");
        Self {
            passes,
            beam: Vec::new().into(),
            grow: Vec::new().into(),
            shrink: Vec::new().into(),
            coin5: Vec::new().into(),
            coin10: Vec::new().into(),
            goal: Vec::new().into(),
            jump: Vec::new().into(),
            shadow: None,
            background_enabled: true,
            textures_enabled: true,
            shadow_enabled: true,
            reflection_enabled: true,
            fov: 50.,
            course,
            ball: ball.into(),
            ball_inner: Vec::new().into(),
            ball_outer: Vec::new().into(),
            ball_file: "ball/basic-ball/basic-ball".into(),
            filtering: None,
            coin: coin.into(),
            background,
            background_data: Vec::new(),
            sky_model: Vec::new().into(),
            animation_time: 0.,
            goal_height: 0.,
            sky: tex!("back/land.png"),
            backdrop_material,
            mirror_backdrop,
            mirror_mask,
        }
    }
    pub async fn load(sol: &Sol, root: &str) -> Result<Self, String> {
        let mut scene = Self::new();
        scene.course = sol_parts(sol, root).await?;
        scene.ball = model(root, "ball/basic-ball/basic-ball-solid.sol").await?;
        scene.coin = model(root, "item/coin/coin.sol").await?;
        scene.coin5 = model(root, "item/coin/coin5.sol").await?;
        scene.coin10 = model(root, "item/coin/coin10.sol").await?;
        scene.grow = model(root, "item/grow/grow.sol").await?;
        scene.shrink = model(root, "item/shrink/shrink.sol").await?;
        scene.goal = model(root, "geom/goal/goal.sol").await?;
        scene.jump = model(root, "geom/jump/jump.sol").await?;
        scene.beam = model(root, "geom/beam/beam.sol").await?;
        scene.shadow = Some(
            load_texture(&format!("{root}/png/shadow.png"))
                .await
                .map_err(|e| e.to_string())?,
        );
        if let Some(gradient) = sol.metadata.get("grad") {
            scene.sky = load_texture(&format!("{root}/{gradient}"))
                .await
                .map_err(|e| format!("{gradient}: {e}"))?;
        }
        let sky_bytes = load_file(&format!("{root}/geom/back/back.sol"))
            .await
            .map_err(|e| e.to_string())?;
        let mut sky_sol = Sol::from_bytes(&sky_bytes).map_err(|e| e.to_string())?;
        for m in &mut sky_sol.materials {
            m.texture = "back/land".into();
        }
        scene.sky_model = Model {
            parts: sol_parts(&sky_sol, root).await?,
            paths: PathRuntime::new(&sky_sol),
            sol: sky_sol,
            elapsed: 0.,
        };
        for p in &mut scene.sky_model.parts {
            p.mesh.texture = Some(scene.sky.clone());
            for v in &mut p.mesh.vertices {
                v.color = WHITE.into();
            }
        }
        scene.background.clear();
        scene.background_data.clear();
        if let Some(background) = sol.metadata.get("back") {
            let bytes = load_file(&format!("{root}/{background}"))
                .await
                .map_err(|e| format!("{background}: {e}"))?;
            let backdrop = Sol::from_bytes(&bytes).map_err(|e| e.to_string())?;
            let textures = sol_textures(&backdrop, root).await?;
            for b in &backdrop.billboards {
                let Some(texture) = textures.get(b.material as usize).and_then(Option::as_ref)
                else {
                    continue;
                };
                let a = &b.parameters;
                let q = Quat::from_rotation_y(a[11].to_radians())
                    * Quat::from_rotation_x(a[8].to_radians())
                    * Quat::from_rotation_z(a[14].to_radians());
                let vertices = [
                    (-0.5, 1., 0., 0.),
                    (0.5, 1., 1., 0.),
                    (0.5, 0., 1., 1.),
                    (-0.5, 0., 0., 1.),
                ]
                .iter()
                .map(|&(x, y, u, v)| {
                    Vertex::new2(q * vec3(x * a[2], y * a[5], -a[1]), vec2(u, v), WHITE)
                })
                .collect();
                scene.background_data.push(b.clone());
                scene.background.push(Mesh {
                    vertices,
                    indices: vec![0, 1, 2, 0, 2, 3],
                    texture: Some(texture.clone()),
                });
            }
        }
        Ok(scene)
    }
    pub fn set_filtering(&mut self, linear: bool) {
        if self.filtering == Some(linear) {
            return;
        }
        self.filtering = Some(linear);
        let filter = if linear {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        };
        for part in &self.course {
            if let Some(t) = &part.mesh.texture {
                t.set_filter(filter);
            }
        }
        for model in [
            &self.ball,
            &self.ball_inner,
            &self.ball_outer,
            &self.coin,
            &self.coin5,
            &self.coin10,
            &self.grow,
            &self.shrink,
            &self.goal,
            &self.jump,
            &self.beam,
        ] {
            for part in &model.parts {
                if let Some(t) = &part.mesh.texture {
                    t.set_filter(filter);
                }
            }
        }
        for mesh in &self.background {
            if let Some(t) = &mesh.texture {
                t.set_filter(filter);
            }
        }
        self.sky.set_filter(filter);
    }
    pub fn ball_preview(&mut self, time: f32) {
        let mut camera = crate::camera::CameraRig::new(Vec3::ZERO);
        camera.eye = vec3(0., 0.4, 3.5);
        camera.target = Vec3::ZERO;
        set_camera(&Camera3D {
            position: camera.eye,
            target: camera.target,
            up: Vec3::Y,
            fovy: 50f32.to_radians(),
            viewport: Some((
                (screen_width() * 0.3) as i32,
                (screen_height() * 0.25) as i32,
                (screen_width() * 0.4) as i32,
                (screen_height() * 0.4) as i32,
            )),
            ..Default::default()
        });
        let q = Quat::from_rotation_y(time * 0.5);
        ball_layer(
            &mut self.ball_outer,
            time,
            Quat::IDENTITY,
            Vec3::ZERO,
            1.,
            true,
            &camera,
            &self.passes,
        );
        ball_layer(
            &mut self.ball,
            time,
            q,
            Vec3::ZERO,
            1.,
            true,
            &camera,
            &self.passes,
        );
        ball_layer(
            &mut self.ball_inner,
            time,
            Quat::IDENTITY,
            Vec3::ZERO,
            1.,
            true,
            &camera,
            &self.passes,
        );
        ball_layer(
            &mut self.ball_inner,
            time,
            Quat::IDENTITY,
            Vec3::ZERO,
            1.,
            false,
            &camera,
            &self.passes,
        );
        ball_layer(
            &mut self.ball,
            time,
            q,
            Vec3::ZERO,
            1.,
            false,
            &camera,
            &self.passes,
        );
        ball_layer(
            &mut self.ball_outer,
            time,
            Quat::IDENTITY,
            Vec3::ZERO,
            1.,
            false,
            &camera,
            &self.passes,
        );
        polygon_offset(false);
        gl_use_default_material();
        set_default_camera();
    }
    pub async fn set_ball(&mut self, root: &str, file: &str) -> Result<(), String> {
        if file == self.ball_file {
            return Ok(());
        }
        let bytes = load_file(&format!("{root}/ball-index.json"))
            .await
            .map_err(|e| e.to_string())?;
        let files: Vec<String> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let mut layers = Vec::new();
        for suffix in ["solid", "inner", "outer"] {
            let path = format!("{file}-{suffix}.sol");
            layers.push(if files.contains(&path) {
                model(root, &path).await?
            } else {
                Vec::new().into()
            });
        }
        if layers.iter().all(|m| m.parts.is_empty()) {
            return Err(format!("No ball model {file}"));
        }
        // Finish bindings that may still reference the avatar being replaced.
        unsafe {
            let mut gl = get_internal_gl();
            gl.flush();
            gl.quad_context.commit_frame();
        }
        self.ball_outer = layers.pop().unwrap();
        self.ball_inner = layers.pop().unwrap();
        self.ball = layers.pop().unwrap();
        self.ball_file = file.into();
        self.filtering = None;
        Ok(())
    }
    pub fn draw(&mut self, g: &FullGame, camera: &crate::camera::CameraRig) {
        // Executable FUN_00036eb0: jump projection contracts around the midpoint.
        // This renderer samples the simulation clock; the reference interpolates client time.
        let configured_fov = self.fov;
        if let Some(jump) = &g.jump {
            self.fov *= 2. * (jump.elapsed - 0.5).abs();
            // A zero-angle perspective is singular; preserve sub-degree values.
            self.fov = self.fov.max(f32::EPSILON);
        }
        let tilt = Quat::from_axis_angle(camera.right, -g.tilt.y.to_radians())
            * Quat::from_axis_angle(camera.back, -g.tilt.x.to_radians());
        let normal = tilt * Vec3::Y;
        let origin = g.ball.position - tilt * g.ball.position;
        for (i, p) in self.passes.iter().enumerate() {
            if let Some(texture) = &self.shadow {
                p.set_texture("Shadow", texture.clone());
            }
            p.set_uniform(
                "ShadowEnabled",
                if self.shadow_enabled { 1f32 } else { 0f32 },
            );
            p.set_uniform("ShadowCenter", g.ball.position);
            p.set_uniform("ShadowX", tilt * Vec3::X);
            p.set_uniform("ShadowY", tilt * Vec3::Y);
            p.set_uniform("ShadowZ", tilt * Vec3::Z);
            p.set_uniform("ShadowScale", 0.25 / g.ball.radius);
            p.set_uniform("ScreenSize", vec2(screen_width(), screen_height()));
            p.set_uniform("PointSize", screen_height() / 6.);

            p.set_uniform("GoalLight", 0f32);
            p.set_uniform(
                "GoalDirection",
                vec3(self.animation_time.cos(), 0., self.animation_time.sin()),
            );
            p.set_uniform(
                "TextureEnabled",
                if self.textures_enabled { 1f32 } else { 0f32 },
            );
            p.set_uniform("ReflectionEnabled", 1f32);
            p.set_uniform("MirrorEnabled", if i >= 32 { 1f32 } else { 0f32 });
            p.set_uniform("MirrorNormal", normal);
            p.set_uniform("MirrorOrigin", origin);
        }
        unsafe {
            let mut gl = get_internal_gl();
            gl.flush();
            gl.quad_context
                .begin_default_pass(miniquad::PassAction::Clear {
                    color: None,
                    depth: Some(1.),
                    stencil: Some(0),
                });
            gl.quad_context.end_render_pass();
        }
        clear_background(SKYBLUE);
        self.draw_background(camera, false, normal);
        if self.reflection_enabled && self.course.iter().any(|p| p.flags & 0x100 != 0) {
            set_camera(&Camera3D {
                position: camera.eye,
                target: camera.target,
                up: Vec3::Y,
                fovy: self.fov.to_radians(),
                z_near: 0.05,
                z_far: 500.,
                ..Default::default()
            });
            gl_use_material(&self.mirror_mask);
            for p in &mut self.course {
                if p.flags & 0x100 != 0 {
                    let pose = g.body_poses.get(p.body).copied().unwrap_or_default();
                    draw_part(
                        p,
                        tilt * pose.rotation,
                        g.ball.position + tilt * (pose.translation - g.ball.position),
                        1.,
                    );
                }
            }
            self.draw_background(camera, true, normal);
            self.draw_world(g, camera, true);
        }
        self.draw_world(g, camera, false);
        self.fov = configured_fov;
    }
    fn draw_background(
        &mut self,
        camera: &crate::camera::CameraRig,
        reflected: bool,
        normal: Vec3,
    ) {
        let eye = camera.eye;
        let target = camera.target;
        set_camera(&Camera3D {
            position: Vec3::ZERO,
            target: target - eye,
            up: Vec3::Y,
            fovy: self.fov.to_radians(),
            z_near: 0.05,
            z_far: 500.,
            ..Default::default()
        });
        let material = if reflected {
            &self.mirror_backdrop
        } else {
            &self.backdrop_material
        };
        material.set_uniform("MirrorEnabled", if reflected { 1f32 } else { 0f32 });
        material.set_uniform("MirrorNormal", normal);
        material.set_uniform("MirrorOrigin", Vec3::ZERO);
        gl_use_material(material);
        if self.background_enabled {
            for p in &mut self.sky_model.parts {
                for (v, base) in p.mesh.vertices.iter_mut().zip(&p.positions) {
                    v.position = *base * vec3(-256., 256., -256.);
                }
                draw_mesh(&p.mesh);
            }
            for (mesh, b) in self.background.iter_mut().zip(&self.background_data) {
                let a = &b.parameters;
                if !(0.0..512.).contains(&a[1]) {
                    continue;
                }
                let t = if a[0] > 0. {
                    self.animation_time % a[0] - a[0] * 0.5
                } else {
                    0.
                };
                let poly = |i: usize| a[i] + a[i + 1] * t + a[i + 2] * t * t;
                let (w, h, rx, ry, rz) = (poly(2), poly(5), poly(8), poly(11), poly(14));
                if w <= 0. || h <= 0. {
                    continue;
                }
                let initial =
                    Quat::from_rotation_y(ry.to_radians()) * Quat::from_rotation_x(rx.to_radians());
                let mut facing = Quat::IDENTITY;
                if b.flags & 2 != 0 {
                    facing *= Quat::from_rotation_x((-90. - rx).to_radians())
                        * Quat::from_rotation_z(-ry.to_radians());
                }
                if b.flags & 1 != 0 {
                    facing *= Quat::from_rotation_x(-rx.to_radians());
                }
                facing *= Quat::from_rotation_z(rz.to_radians());
                let bottom = if b.flags & 1 != 0 { 0. } else { -0.5 };
                for (v, (x, y)) in mesh.vertices.iter_mut().zip([
                    (-0.5, bottom + 1.),
                    (0.5, bottom + 1.),
                    (0.5, bottom),
                    (-0.5, bottom),
                ]) {
                    v.position = initial * (vec3(0., 0., -a[1]) + facing * vec3(x * w, y * h, 0.));
                }
                draw_mesh(mesh);
            }
        }
        polygon_offset(false);
        gl_use_default_material();
    }
    fn draw_world(&mut self, g: &FullGame, camera: &crate::camera::CameraRig, reflected: bool) {
        let passes = if reflected {
            self.passes[32..].to_vec()
        } else {
            self.passes[..32].to_vec()
        };
        polygon_offset(false);
        gl_use_default_material();
        set_camera(&Camera3D {
            position: camera.eye,
            target: camera.target,
            up: Vec3::Y,
            fovy: self.fov.to_radians(),
            z_near: 0.05,
            z_far: 500.,
            ..Default::default()
        });
        let tilt = Quat::from_axis_angle(camera.right, -g.tilt.y.to_radians())
            * Quat::from_axis_angle(camera.back, -g.tilt.x.to_radians());
        for p in &mut self.course {
            if reflected && p.flags & 0x100 != 0 {
                continue;
            }
            use_part_material(p, camera, &passes);
            if p.billboard.is_some() {
                draw_billboard(
                    p,
                    self.animation_time,
                    tilt,
                    g.ball.position - tilt * g.ball.position,
                    Vec3::ONE,
                    camera,
                );
                continue;
            }
            let pose = g.body_poses.get(p.body).copied().unwrap_or_default();
            draw_part(
                p,
                tilt * pose.rotation,
                g.ball.position + tilt * (pose.translation - g.ball.position),
                1.,
            );
        }
        for (i, item) in g.sol.items.iter().enumerate() {
            if !g.collected[i] {
                let pos =
                    g.ball.position + tilt * (Vec3::from_array(item.position) - g.ball.position);
                let model = match item.kind {
                    2 => &mut self.grow,
                    3 => &mut self.shrink,
                    _ => {
                        if item.value >= 10 {
                            &mut self.coin10
                        } else if item.value >= 5 {
                            &mut self.coin5
                        } else {
                            &mut self.coin
                        }
                    }
                };
                draw_model(
                    model,
                    g.elapsed,
                    tilt,
                    pos,
                    Vec3::splat(0.15),
                    None,
                    false,
                    camera,
                    &passes,
                );
            }
        }
        polygon_offset(false);
        gl_use_default_material();
        // Executable FUN_1f520/FUN_13440 nesting: outer rear, solid rear,
        // inner, solid front, outer front. Secondary layers may be pendulums.
        let pos = g.ball.position + vec3(0., 0.001, 0.);
        let outer_q = if metadata_flag(&self.ball_outer, "pendulum", false) {
            g.ball.inner_orientation
        } else {
            Quat::IDENTITY
        };
        let inner_q = if metadata_flag(&self.ball_inner, "pendulum", false) {
            g.ball.inner_orientation
        } else {
            Quat::IDENTITY
        };
        ball_layer(
            &mut self.ball_outer,
            g.elapsed,
            outer_q,
            pos,
            g.ball.radius,
            true,
            camera,
            &passes,
        );
        ball_layer(
            &mut self.ball,
            g.elapsed,
            g.ball.orientation,
            pos,
            g.ball.radius,
            true,
            camera,
            &passes,
        );
        ball_layer(
            &mut self.ball_inner,
            g.elapsed,
            inner_q,
            pos,
            g.ball.radius,
            true,
            camera,
            &passes,
        );
        ball_layer(
            &mut self.ball_inner,
            g.elapsed,
            inner_q,
            pos,
            g.ball.radius,
            false,
            camera,
            &passes,
        );
        ball_layer(
            &mut self.ball,
            g.elapsed,
            g.ball.orientation,
            pos,
            g.ball.radius,
            false,
            camera,
            &passes,
        );
        ball_layer(
            &mut self.ball_outer,
            g.elapsed,
            outer_q,
            pos,
            g.ball.radius,
            false,
            camera,
            &passes,
        );
        if g.goal_open() {
            for goal in &g.sol.goals {
                let center =
                    g.ball.position + tilt * (vec3(goal[0], goal[1], goal[2]) - g.ball.position);
                draw_model(
                    &mut self.beam,
                    g.elapsed,
                    tilt,
                    center,
                    vec3(goal[3], 3. * self.goal_height, goal[3]),
                    Some([1., 1., 0., 0.5]),
                    true,
                    camera,
                    &passes,
                );
            }
        }
        for jump in &g.sol.jumps {
            for p in &passes {
                p.set_uniform("PointSize", screen_height() / 12.);
            }
            let center =
                g.ball.position + tilt * (vec3(jump[0], jump[1], jump[2]) - g.ball.position);
            draw_model(
                &mut self.beam,
                g.elapsed,
                tilt,
                center,
                vec3(jump[6], 2., jump[6]),
                Some([0.7, 0.5, 1., if g.jump_enabled { 0.5 } else { 0.8 }]),
                true,
                camera,
                &passes,
            );
        }
        for (i, switch) in g.sol.switches.iter().enumerate() {
            if switch.words[2] != 0 {
                continue;
            }
            let state = &g.switches[i];
            let alpha = if state.entered { 0.8 } else { 0.5 };
            let color = if state.enabled {
                [0., 1., 0., alpha]
            } else {
                [1., 0., 0., alpha]
            };
            let center =
                g.ball.position + tilt * (Vec3::from_array(switch.position) - g.ball.position);
            draw_model(
                &mut self.beam,
                g.elapsed,
                tilt,
                center,
                vec3(switch.radius, 2., switch.radius),
                Some(color),
                true,
                camera,
                &passes,
            );
        }
        for p in &passes {
            p.set_uniform("GoalLight", 1f32);
            p.set_uniform("PointSize", screen_height() / 6.);
        }
        if g.goal_open() {
            for goal in &g.sol.goals {
                let center =
                    g.ball.position + tilt * (vec3(goal[0], goal[1], goal[2]) - g.ball.position);
                draw_model(
                    &mut self.goal,
                    g.elapsed,
                    tilt,
                    center,
                    vec3(goal[3], self.goal_height, goal[3]),
                    None,
                    true,
                    camera,
                    &passes,
                );
            }
        }
        for p in &passes {
            p.set_uniform("PointSize", screen_height() / 12.);
        }
        for jump in &g.sol.jumps {
            let center =
                g.ball.position + tilt * (vec3(jump[0], jump[1], jump[2]) - g.ball.position);
            draw_model(
                &mut self.jump,
                g.elapsed,
                tilt,
                center,
                vec3(jump[6], 1., jump[6]),
                None,
                true,
                camera,
                &passes,
            );
        }
        for p in &passes {
            p.set_uniform("GoalLight", 0f32);
        }
        polygon_offset(false);
        gl_use_default_material();
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

async fn sol_textures(sol: &Sol, root: &str) -> Result<Vec<Option<Texture2D>>, String> {
    static INDEX: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    if INDEX.get().is_none() {
        let bytes = load_file(&format!("{root}/texture-index.json"))
            .await
            .map_err(|e| e.to_string())?;
        let index = serde_json::from_slice(&bytes).map_err(|e| format!("Texture index: {e}"))?;
        let _ = INDEX.set(index);
    }
    let index = INDEX.get().unwrap();
    let mut textures = Vec::new();
    for m in &sol.materials {
        if m.diffuse[3] == 0. {
            textures.push(None);
            continue;
        }
        // This one material uses independently captured runtime pixels. Originals
        // and the public texture index are retained; see runtime-artwork provenance.
        let path = if m.texture == "ball/basic-ball/basic-ball" {
            "runtime-artwork/basic-ball-captured.png"
        } else {
            index
                .get(&m.texture)
                .ok_or_else(|| format!("Missing indexed texture {}", m.texture))?
                .as_str()
        };
        let t = load_texture(&format!("{root}/{path}"))
            .await
            .map_err(|e| e.to_string())?;
        t.set_filter(FilterMode::Linear);
        textures.push(Some(t));
    }
    Ok(textures)
}
async fn sol_parts(sol: &Sol, root: &str) -> Result<Vec<Part>, String> {
    use std::collections::BTreeSet;
    let textures = sol_textures(sol, root).await?;
    let mut result = Vec::new();
    let indexed = |start: i32, count: i32| -> Vec<usize> {
        sol.indices
            .iter()
            .skip(start.max(0) as usize)
            .take(count.max(0) as usize)
            .filter(|&&x| x >= 0)
            .map(|&x| x as usize)
            .collect()
    };
    for (body_id, b) in sol.bodies.iter().enumerate() {
        let mut triangles = BTreeSet::new();
        triangles.extend(indexed(b[5], b[6]));
        for l in sol
            .lumps
            .iter()
            .skip(b[3].max(0) as usize)
            .take(b[4].max(0) as usize)
        {
            triangles.extend(indexed(l[5], l[6]));
        }
        for (mi, texture) in textures.iter().enumerate() {
            let Some(texture) = texture else {
                continue;
            };
            let ids: Vec<_> = triangles
                .iter()
                .filter_map(|&i| sol.triangles.get(i))
                .filter(|t| t[0] as usize == mi)
                .collect();
            for chunk in ids.chunks(180) {
                let mut vertices = Vec::new();
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                for t in chunk {
                    for &corner in &t[1..] {
                        let c = sol.corners[corner as usize];
                        let p = Vec3::from_array(sol.vertices[c[2] as usize]);
                        let n = sol.planes[c[1] as usize];
                        let n = vec3(n[0], n[1], n[2]);
                        let uv = sol.uv[c[0] as usize];
                        let color = if sol.materials[mi].flags & 0x800 != 0 {
                            Color::from(sol.materials[mi].diffuse)
                        } else {
                            WHITE
                        };
                        let mut v = Vertex::new2(p, vec2(uv[0], 1. - uv[1]), color);
                        v.normal = n.extend(0.);
                        vertices.push(v);
                        positions.push(p);
                        normals.push(n);
                    }
                }
                let mut indices: Vec<u16> = (0..vertices.len() as u16).collect();
                if sol.materials[mi].flags & 0x400 != 0 {
                    let mut unique = std::collections::BTreeSet::new();
                    let mut points = Vec::new();
                    for (v, n) in vertices.iter().zip(&normals) {
                        let key = [
                            v.position.x.to_bits(),
                            v.position.y.to_bits(),
                            v.position.z.to_bits(),
                            n.x.to_bits(),
                            n.y.to_bits(),
                            n.z.to_bits(),
                            v.uv.x.to_bits(),
                            v.uv.y.to_bits(),
                        ];
                        if unique.insert(key) {
                            points.push((*v, *n));
                        }
                    }
                    vertices.clear();
                    positions.clear();
                    normals.clear();
                    indices.clear();
                    for (v, n) in points {
                        let start = vertices.len() as u16;
                        for uv in [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)] {
                            let mut corner = v;
                            corner.uv = uv;
                            vertices.push(corner);
                            positions.push(v.position);
                            normals.push(n);
                        }
                        indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);
                    }
                }
                result.push(Part {
                    billboard: None,
                    flags: sol.materials[mi].flags,
                    shine: sol.materials[mi].shininess,
                    ambient: Vec4::from_array(sol.materials[mi].ambient).truncate(),
                    specular: Vec4::from_array(sol.materials[mi].specular).truncate(),
                    emission: Vec4::from_array(sol.materials[mi].emission).truncate(),
                    alpha_clip: sol.materials[mi]
                        .optional
                        .filter(|x| x.0 == 2)
                        .map_or(-1., |x| x.1),
                    body: body_id,
                    mesh: Mesh {
                        indices,
                        vertices,
                        texture: Some(texture.clone()),
                    },
                    positions,
                    normals,
                });
            }
        }
    }
    for b in &sol.billboards {
        let mi = b.material as usize;
        let Some(texture) = textures.get(mi).and_then(Option::as_ref) else {
            continue;
        };
        let m = &sol.materials[mi];
        let color = if m.flags & 0x800 != 0 {
            Color::from(m.diffuse)
        } else {
            WHITE
        };
        let vertices = [vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)]
            .into_iter()
            .map(|uv| Vertex::new2(Vec3::ZERO, uv, color))
            .collect();
        result.push(Part {
            billboard: Some(b.clone()),
            flags: (m.flags & !0x400) | 0x80,
            shine: m.shininess,
            ambient: Vec4::from_array(m.ambient).truncate(),
            specular: Vec4::from_array(m.specular).truncate(),
            emission: Vec4::from_array(m.emission).truncate(),
            alpha_clip: m.optional.filter(|x| x.0 == 2).map_or(-1., |x| x.1),
            body: 0,
            mesh: Mesh {
                vertices,
                indices: vec![0, 1, 2, 0, 2, 3],
                texture: Some(texture.clone()),
            },
            positions: vec![Vec3::ZERO; 4],
            normals: vec![Vec3::Z; 4],
        });
    }
    result.sort_by_key(|p| ((p.flags & 0x80 != 0) as u8, (p.flags & 0x20 != 0) as u8));
    Ok(result)
}

async fn model(root: &str, path: &str) -> Result<Model, String> {
    let bytes = load_file(&format!("{root}/{path}"))
        .await
        .map_err(|e| format!("{path}: {e}"))?;
    let sol = Sol::from_bytes(&bytes).map_err(|e| e.to_string())?;
    let parts = sol_parts(&sol, root).await?;
    let paths = PathRuntime::new(&sol);
    Ok(Model {
        parts,
        sol,
        paths,
        elapsed: 0.,
    })
}
#[allow(clippy::too_many_arguments)]
fn draw_model(
    model: &mut Model,
    time: f32,
    q: Quat,
    offset: Vec3,
    scale: Vec3,
    color: Option<[f32; 4]>,
    transparent: bool,
    camera: &crate::camera::CameraRig,
    passes: &[Material],
) {
    let poses = model.at(time);
    for p in &mut model.parts {
        let old = p.flags;
        if transparent {
            p.flags |= 0x80;
        }
        use_part_material(p, camera, passes);
        p.flags = old;
        if p.billboard.is_some() {
            draw_billboard(p, time, q, offset, scale, camera);
            continue;
        }
        let pose = poses.get(p.body).copied().unwrap_or_default();
        for ((v, base), normal) in p.mesh.vertices.iter_mut().zip(&p.positions).zip(&p.normals) {
            v.position = offset + q * (pose.point(*base) * scale);
            v.normal = (q
                * ((pose.rotation * *normal) / scale.max(Vec3::splat(0.001))).normalize())
            .extend(0.);
            if let Some(c) = color {
                v.color = Color::from(c).into();
            }
        }
        draw_mesh(&p.mesh);
    }
}
fn draw_billboard(
    p: &mut Part,
    time: f32,
    q: Quat,
    offset: Vec3,
    scale: Vec3,
    camera: &crate::camera::CameraRig,
) {
    let b = p.billboard.as_ref().unwrap();
    let a = &b.parameters;
    let t = time * a[0];
    let wave = t.sin();
    let value = |i: usize| a[i] + a[i + 1] * t + a[i + 2] * wave;
    let width = value(2);
    let height = value(5);
    let rotation = Quat::from_rotation_x(value(8).to_radians())
        * Quat::from_rotation_y(value(11).to_radians())
        * Quat::from_rotation_z(value(14).to_radians());
    let center = offset + q * (vec3(a[17], a[18], a[19]) * scale);
    let back = (camera.eye - camera.target).normalize_or_zero();
    let right = Vec3::Y.cross(back).normalize_or_zero();
    let up = back.cross(right);
    let facing = if b.flags & 4 != 0 {
        Mat3::from_quat(q)
    } else {
        Mat3::from_cols(right, up, back)
    };
    for (v, (x, y)) in
        p.mesh
            .vertices
            .iter_mut()
            .zip([(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)])
    {
        v.position = center + facing * (rotation * vec3(x * width, y * height, 0.) * scale);
        v.normal = back.extend(0.);
    }
    draw_mesh(&p.mesh);
}
fn use_part_material(p: &Part, camera: &crate::camera::CameraRig, passes: &[Material]) {
    let bits = (p.flags & 0x80 != 0) as usize
        | (((p.flags & 8 != 0) as usize) << 1)
        | (((p.flags & 4 != 0) as usize) << 2)
        | (((p.flags & 0x10000 != 0) as usize) << 3)
        | (((p.flags & 0x20000 != 0) as usize) << 4);
    let material = &passes[bits];
    let back = (camera.eye - camera.target).normalize_or_zero();
    let right = Vec3::Y.cross(back).normalize_or_zero();
    let up = back.cross(right);
    material.set_uniform("Eye", camera.eye);
    material.set_uniform("Gloss", p.shine.clamp(0., 128.));
    material.set_uniform(
        "ShadowReceiver",
        if p.flags & 0x40 != 0 { 1f32 } else { 0f32 },
    );
    polygon_offset(p.flags & 0x20 != 0);
    material.set_uniform(
        "PointSprite",
        if p.flags & 0x400 != 0 { 1f32 } else { 0f32 },
    );
    material.set_uniform("MaterialAmbient", p.ambient);
    material.set_uniform("MaterialSpecular", p.specular);
    material.set_uniform("MaterialEmission", p.emission);
    material.set_uniform("Environment", if p.flags & 0x10 != 0 { 1f32 } else { 0f32 });
    material.set_uniform("Lighting", if p.flags & 0x800 != 0 { 1f32 } else { 0f32 });
    material.set_uniform("AlphaClip", p.alpha_clip);
    material.set_uniform("CameraRight", right);
    material.set_uniform("CameraUp", up);
    material.set_uniform("CameraBack", back);
    gl_use_material(material);
}

fn metadata_flag(model: &Model, key: &str, default: bool) -> bool {
    model
        .sol
        .metadata
        .get(key)
        .and_then(|s| s.parse::<i32>().ok())
        .map_or(default, |v| v != 0)
}
#[allow(clippy::too_many_arguments)] // Explicit model transform and binary pass controls.
fn ball_layer(
    model: &mut Model,
    time: f32,
    q: Quat,
    pos: Vec3,
    radius: f32,
    rear: bool,
    camera: &crate::camera::CameraRig,
    passes: &[Material],
) {
    if rear && !metadata_flag(model, "drawback", false) && !metadata_flag(model, "drawclip", false)
    {
        return;
    }
    let clip = metadata_flag(model, "drawclip", false);
    let depth = metadata_flag(model, "depthmask", false);
    let test = metadata_flag(model, "depthtest", true);
    let flags: Vec<_> = model.parts.iter().map(|p| p.flags).collect();
    for p in &mut model.parts {
        if depth {
            p.flags &= !0x80;
        } else {
            p.flags |= 0x80;
        }
        if clip {
            p.flags |= 8;
        } else if rear {
            p.flags |= 0x10000;
        }
        if !test {
            p.flags |= 0x20000;
        }
    }
    for p in passes {
        p.set_uniform(
            "ClipHalf",
            if clip {
                if rear {
                    -1f32
                } else {
                    1f32
                }
            } else {
                0f32
            },
        );
        p.set_uniform("ClipCenter", pos);
        p.set_uniform("ClipNormal", (camera.eye - pos).normalize_or_zero());
    }
    draw_model(
        model,
        time,
        q,
        pos,
        Vec3::splat(radius),
        None,
        false,
        camera,
        passes,
    );
    for p in passes {
        p.set_uniform("ClipHalf", 0f32);
    }
    for (p, f) in model.parts.iter_mut().zip(flags) {
        p.flags = f;
    }
}

fn stencil(write: bool) -> miniquad::StencilState {
    let face = miniquad::StencilFaceState {
        fail_op: miniquad::StencilOp::Keep,
        depth_fail_op: miniquad::StencilOp::Keep,
        pass_op: if write {
            miniquad::StencilOp::Replace
        } else {
            miniquad::StencilOp::Keep
        },
        test_func: if write {
            miniquad::CompareFunc::Always
        } else {
            miniquad::CompareFunc::Equal
        },
        test_ref: 1,
        test_mask: u32::MAX,
        write_mask: u32::MAX,
    };
    miniquad::StencilState {
        front: face,
        back: face,
    }
}
