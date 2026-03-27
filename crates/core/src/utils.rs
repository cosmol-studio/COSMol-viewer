use glam::{Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};

use crate::shapes::{Molecule, Protein, Sphere, SphereInstance, Stick, StickInstance};

pub trait Logger: Send + Sync + Copy {
    fn log(&self, message: impl std::fmt::Display);
    fn error(&self, message: impl std::fmt::Display);
    fn warn(&self, message: impl std::fmt::Display);
}

#[derive(Clone, Copy)]
pub struct RustLogger;

impl Logger for RustLogger {
    fn log(&self, message: impl std::fmt::Display) {
        println!("[LOG] {}", message);
    }
    fn warn(&self, message: impl std::fmt::Display) {
        eprintln!("[WARN] {}", message);
    }
    fn error(&self, message: impl std::fmt::Display) {
        eprintln!("[ERROR] {}", message);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct VisualStyle {
    pub color: Option<Vec3>,
    pub opacity: f32,
    pub wireframe: bool,
    pub visible: bool,
    pub line_width: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct Interaction {
    pub clickable: bool,
    pub hoverable: bool,
    pub context_menu_enabled: bool,
    // 可扩展为事件 enum，如 Click(EventCallback)
}

pub trait Interpolatable {
    /// t ∈ [0.0, 1.0]，返回两个实例之间的插值
    fn interpolate(&self, other: &Self, t: f32, logger: impl Logger) -> Self;
}

// -------------------- 图元结构体 --------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Shape {
    Sphere(Sphere),
    Stick(Stick),
    Molecules(Molecule),
    Protein(Protein),
    Qudrate, // Custom(CustomShape),
             // ...
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShapeKind {
    Sphere,
    Stick,
}

pub struct InstanceData {
    pub position: [f32; 3],
    pub scale: f32, // 比如 Sphere 半径或 Cylinder 长度
    pub color: [f32; 4],
    pub extra: Option<[f32; 3]>, // 比如 Cylinder 要方向向量
}

impl Interpolatable for Shape {
    fn interpolate(&self, other: &Self, t: f32, logger: impl Logger) -> Self {
        match (self, other) {
            (Shape::Sphere(a), Shape::Sphere(b)) => Shape::Sphere(a.interpolate(b, t, logger)),
            (Shape::Stick(a), Shape::Stick(b)) => Shape::Stick(a.interpolate(b, t, logger)),
            (Shape::Molecules(a), Shape::Molecules(b)) => {
                Shape::Molecules(a.interpolate(b, t, logger))
            }
            _ => self.clone(), // 如果类型不匹配，可以选择不插值或做默认处理
        }
    }
}

#[derive(Clone, Default)]
pub struct InstanceGroups {
    pub spheres: Vec<SphereInstance>,
    pub sticks: Vec<StickInstance>,
}

impl InstanceGroups {
    pub fn merge(&mut self, mut other: InstanceGroups) {
        self.spheres.append(&mut other.spheres);
        self.sticks.append(&mut other.sticks);
    }
}

impl IntoInstanceGroups for Shape {
    fn to_instance_group(&self, scale: f32) -> InstanceGroups {
        let mut groups = InstanceGroups::default();

        match self {
            Shape::Sphere(s) => {
                groups.spheres.push(s.to_instance(scale));
            }
            Shape::Molecules(m) => {
                let m_groups = m.to_instance_group(scale);
                groups.merge(m_groups);
            }
            _ => {}
        }
        groups
    }
}

pub trait IntoInstanceGroups {
    fn to_instance_group(&self, scale: f32) -> InstanceGroups;
}

pub trait ToMesh {
    fn to_mesh(&self, scale: f32) -> MeshData;
}

impl ToMesh for Shape {
    fn to_mesh(&self, scale: f32) -> MeshData {
        match self {
            Shape::Sphere(s) => s.to_mesh(scale),
            Shape::Stick(s) => s.to_mesh(scale),
            Shape::Molecules(s) => s.to_mesh(scale),
            Shape::Protein(s) => s.to_mesh(scale),
            Shape::Qudrate => todo!(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub colors: Option<Vec<Vec4>>,
    pub transform: Option<Mat4>, // 可选位移旋转缩放
    pub is_wireframe: bool,
}

impl MeshData {
    /// Append another MeshData into this one.
    pub fn append(&mut self, other: &MeshData) {
        let base = self.vertices.len() as u32;

        // append vertices
        self.vertices.extend(&other.vertices);

        // append normals
        self.normals.extend(&other.normals);

        // append colors
        if let Some(ref mut my_colors) = self.colors {
            if let Some(ref other_colors) = other.colors {
                my_colors.extend(other_colors);
            }
        } else if let Some(ref other_colors) = other.colors {
            self.colors = Some(other_colors.clone());
        }

        // append indices with offset
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color(pub Vec3);

impl From<Vec3> for Color {
    fn from(c: Vec3) -> Self {
        Color(c)
    }
}

impl From<[u8; 3]> for Color {
    fn from(c: [u8; 3]) -> Self {
        Color(Vec3::new(
            c[0] as f32 / 255.0,
            c[1] as f32 / 255.0,
            c[2] as f32 / 255.0,
        ))
    }
}

impl From<&str> for Color {
    fn from(s: &str) -> Self {
        let s = s.trim_start_matches('#');

        assert!(s.len() == 6, "HEX color must be 6 characters");

        let r = u8::from_str_radix(&s[0..2], 16).unwrap();
        let g = u8::from_str_radix(&s[2..4], 16).unwrap();
        let b = u8::from_str_radix(&s[4..6], 16).unwrap();

        Color::from([r, g, b])
    }
}

impl Into<Vec3> for Color {
    fn into(self) -> Vec3 {
        self.0
    }
}

pub trait VisualShape {
    fn style_mut(&mut self) -> &mut VisualStyle;

    fn color<C: Into<Color>>(mut self, color: C) -> Self
    where
        Self: Sized,
    {
        self.style_mut().color = Some(color.into().into());
        self
    }

    fn color_rgba(mut self, color: [f32; 4]) -> Self
    where
        Self: Sized,
    {
        self.style_mut().color = Some(Vec3::new(color[0], color[1], color[2]));
        self.style_mut().opacity = color[3];

        self
    }

    fn opacity(mut self, opacity: f32) -> Self
    where
        Self: Sized,
    {
        self.style_mut().opacity = opacity;
        self
    }
}

pub enum RenderQuality {
    Low,
    Medium,
    High,
    Ultra,
}
