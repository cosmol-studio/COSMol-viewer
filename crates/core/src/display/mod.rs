//! Visual projections of CK values. Only geometry crosses the renderer transport.
//! Native scenes retain shared CK value snapshots; subsequent scientific edits
//! never retarget a display. Each add call creates an independent projection.
//!
//! Scientific construction belongs exclusively to CK; old viewer chemical
//! wrappers have been removed, rather than retained as compatibility aliases:
//!
//! ```compile_fail,E0432
//! use cosmol_viewer_core::shapes::{Molecule, Protein};
//! ```

pub mod input;
mod molecular;
mod protein;

use crate::styles::Representation;
use crate::utils::{InstanceGroups, Interpolatable, Logger, MeshData};
use cosmolkit as ck;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("positioned shapes already carry their style; omit the representation argument")]
    ShapeStyle,
    #[error("radius must be finite and positive")]
    InvalidRadius,
    #[error("selection is not available for this source/entity kind")]
    InvalidSelection,
    #[error("unknown atomic number {0}")]
    InvalidElement(u8),
    #[error("selection belongs to a different source object")]
    SourceMismatch,
    #[error("{kind} ID {index} does not exist in the source")]
    InvalidEntity { kind: &'static str, index: usize },
    #[error("{style} does not support {selection}")]
    IncompatibleStyle {
        style: &'static str,
        selection: &'static str,
    },
    #[error("source has no stored coordinates; generate them explicitly with COSMolKit")]
    MissingCoordinates,
    #[error("source has {0} 3D conformers; select an explicit conformer ID")]
    AmbiguousCoordinates(usize),
    #[error("source has no 3D conformer with ID {0}")]
    MissingConformer(usize),
    #[error("source contains coordinates that cannot be represented as finite f32 values")]
    InvalidCoordinates,
    #[error("source has {atoms} atoms but {coordinates} coordinate rows")]
    CoordinateCount { atoms: usize, coordinates: usize },
    #[error("cannot draw residue {0}: N, CA, C and O backbone atoms are required")]
    MissingBackbone(u32),
    #[error(transparent)]
    Surface(#[from] crate::surface::SurfaceError),
}

enum SourceSnapshot {
    Molecule(ck::Molecule),
    Protein(ck::Protein),
}

/// Render geometry, not a molecule/protein model. Its retained CK snapshot is
/// private and shared on clone; transport serializes only the visual projection.
#[derive(Clone, Serialize, Deserialize)]
pub struct Display {
    pub(crate) mesh: MeshData,
    pub(crate) instances: InstanceGroups,
    center: Vec3,
    #[serde(skip)]
    source: Option<Arc<SourceSnapshot>>,
}

impl std::fmt::Debug for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_atoms = self.source.as_deref().map(|source| match source {
            SourceSnapshot::Molecule(source) => source.num_atoms(),
            SourceSnapshot::Protein(source) => source.num_atoms(),
        });
        f.debug_struct("Display")
            .field("source_atoms", &source_atoms)
            .field("mesh_vertices", &self.mesh.vertices.len())
            .field("spheres", &self.instances.spheres.len())
            .field("sticks", &self.instances.sticks.len())
            .field("center", &self.center)
            .finish()
    }
}

impl Display {
    /// Geometric center used for camera framing, without moving scientific data.
    pub fn center(&self) -> [f32; 3] {
        self.center.to_array()
    }

    pub(crate) fn mesh(&self, scale: f32) -> MeshData {
        let mut mesh = self.mesh.clone();
        for vertex in &mut mesh.vertices {
            *vertex *= scale;
        }
        mesh
    }

    pub(crate) fn instances(&self, scale: f32) -> InstanceGroups {
        let mut groups = self.instances.clone();
        fn spheres(spheres: &mut [crate::shapes::SphereInstance], scale: f32) {
            for sphere in spheres {
                sphere.position = sphere.position.map(|x| x * scale);
                sphere.radius *= scale;
            }
        }
        fn sticks(sticks: &mut [crate::shapes::StickInstance], scale: f32) {
            for stick in sticks {
                stick.start = stick.start.map(|x| x * scale);
                stick.end = stick.end.map(|x| x * scale);
                stick.radius *= scale;
            }
        }
        spheres(&mut groups.spheres, scale);
        sticks(&mut groups.sticks, scale);
        for outline in &mut groups.outlines {
            outline.settings.width *= scale;
            spheres(&mut outline.spheres, scale);
            sticks(&mut outline.sticks, scale);
        }
        groups
    }
}

/// Native projection extension point used by Scene; callers need not import it.
pub trait ProjectSource<I> {
    fn default_style() -> Representation {
        crate::styles::BallAndStick::default().into()
    }
    fn project(&self, ids: &[I], style: Representation) -> Result<Display, DisplayError>;
}

/// Scene content, optionally paired with an explicit scientific representation.
/// Defaults and geometry validation are shared by every language binding.
pub trait SceneItem {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError>;
}
impl<T: SceneItem + ?Sized> SceneItem for &T {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        (**self).to_shape(style)
    }
}
impl<T: SceneItem, S: Into<Representation> + Copy> SceneItem for (T, S) {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        self.0
            .to_shape(Some(style.unwrap_or_else(|| self.1.into())))
    }
}
impl SceneItem for ck::Molecule {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        crate::selection::select(self).atoms().to_shape(style)
    }
}
impl SceneItem for ck::Protein {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        let style = style.unwrap_or_else(|| crate::styles::Cartoon::default().into());
        match style {
            Representation::Cartoon(_) => crate::selection::select(self)
                .residues()
                .to_shape(Some(style)),
            _ => crate::selection::select(self).atoms().to_shape(Some(style)),
        }
    }
}
impl<S: ProjectSource<I> + ?Sized, I> SceneItem for crate::selection::Selection<'_, S, I> {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        self.source()
            .project(self.ids(), style.unwrap_or_else(S::default_style))
            .map(crate::utils::Shape::Display)
    }
}
impl SceneItem for crate::utils::Shape {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        if style.is_some() {
            return Err(DisplayError::ShapeStyle);
        }
        match self {
            Self::Sphere(s) => validate_geometry(&s.center, s.radius)?,
            Self::Stick(s) => {
                validate_geometry(&s.start, s.thickness_radius)?;
                validate_geometry(&s.end, s.thickness_radius)?;
            }
            _ => {}
        }
        Ok(self.clone())
    }
}
fn validate_geometry(position: &[f32; 3], radius: f32) -> Result<(), DisplayError> {
    if !position.iter().all(|v| v.is_finite()) {
        return Err(DisplayError::InvalidCoordinates);
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(DisplayError::InvalidRadius);
    }
    Ok(())
}
impl SceneItem for crate::shapes::Sphere {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        crate::utils::Shape::Sphere(*self).to_shape(style)
    }
}
impl SceneItem for crate::shapes::Stick {
    fn to_shape(&self, style: Option<Representation>) -> Result<crate::utils::Shape, DisplayError> {
        crate::utils::Shape::Stick(*self).to_shape(style)
    }
}

/// A scientific object that supplies a geometric center for scene framing.
/// Callers of `Scene::recenter_on` do not need to import this trait.
pub trait CenterSource {
    /// Unweighted center of all atoms, using stored coordinates without mutation.
    fn center(&self) -> Result<[f32; 3], DisplayError>;
}

impl CenterSource for ck::Molecule {
    fn center(&self) -> Result<[f32; 3], DisplayError> {
        if self.num_atoms() == 0 {
            return Ok([0.0; 3]);
        }
        let positions = molecular::coordinates(
            &input::MoleculeInput::from(self),
            crate::styles::Coordinates::Auto,
        )?;
        Ok(center(positions.into_iter()).to_array())
    }
}

impl CenterSource for ck::Protein {
    fn center(&self) -> Result<[f32; 3], DisplayError> {
        let positions = self
            .atoms()
            .map(|atom| position(atom.position()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(center(positions.into_iter()).to_array())
    }
}

impl ProjectSource<ck::AtomId> for ck::Molecule {
    fn project(&self, ids: &[ck::AtomId], style: Representation) -> Result<Display, DisplayError> {
        let (instances, center) =
            molecular::project_atoms(&input::MoleculeInput::from(self), ids, style)?;
        Ok(Display {
            mesh: MeshData::default(),
            instances,
            center,
            source: Some(Arc::new(SourceSnapshot::Molecule(self.clone()))),
        })
    }
}

impl ProjectSource<ck::BondId> for ck::Molecule {
    fn default_style() -> Representation {
        crate::styles::Stick::default().into()
    }
    fn project(&self, ids: &[ck::BondId], style: Representation) -> Result<Display, DisplayError> {
        let (instances, center) =
            molecular::project_bonds(&input::MoleculeInput::from(self), ids, style)?;
        Ok(Display {
            mesh: MeshData::default(),
            instances,
            center,
            source: Some(Arc::new(SourceSnapshot::Molecule(self.clone()))),
        })
    }
}

impl ProjectSource<ck::BioResidueId> for ck::Protein {
    fn default_style() -> Representation {
        crate::styles::Cartoon::default().into()
    }
    fn project(
        &self,
        ids: &[ck::BioResidueId],
        style: Representation,
    ) -> Result<Display, DisplayError> {
        let (mesh, instances, center) =
            protein::project_residues(&input::ProteinInput::try_from(self)?, ids, style)?;
        Ok(Display {
            mesh,
            instances,
            center,
            source: Some(Arc::new(SourceSnapshot::Protein(self.clone()))),
        })
    }
}

impl ProjectSource<ck::BioAtomId> for ck::Protein {
    fn default_style() -> Representation {
        crate::styles::Sphere::default().into()
    }
    fn project(
        &self,
        ids: &[ck::BioAtomId],
        style: Representation,
    ) -> Result<Display, DisplayError> {
        let (mesh, instances, center) =
            protein::project_atoms(&input::ProteinInput::try_from(self)?, ids, style)?;
        Ok(Display {
            mesh,
            instances,
            center,
            source: Some(Arc::new(SourceSnapshot::Protein(self.clone()))),
        })
    }
}

pub(crate) fn position(row: [f64; 3]) -> Result<Vec3, DisplayError> {
    let position = Vec3::from_array(row.map(|x| x as f32));
    if position.is_finite() {
        Ok(position)
    } else {
        Err(DisplayError::InvalidCoordinates)
    }
}

pub(crate) fn center(positions: impl Iterator<Item = Vec3>) -> Vec3 {
    // Accumulate in f64 so many large finite f32 positions do not overflow.
    let mut sum = [0.0_f64; 3];
    let mut count = 0;
    for position in positions {
        for (sum, value) in sum.iter_mut().zip(position.to_array()) {
            *sum += f64::from(value);
        }
        count += 1;
    }
    if count == 0 {
        Vec3::ZERO
    } else {
        Vec3::from_array(sum.map(|x| (x / count as f64) as f32))
    }
}

impl Interpolatable for Display {
    fn interpolate(&self, other: &Self, t: f32, logger: impl Logger) -> Self {
        let same_layout = self.mesh.indices == other.mesh.indices
            && self.mesh.vertices.len() == other.mesh.vertices.len()
            && self.mesh.normals.len() == other.mesh.normals.len()
            && self.instances.spheres.len() == other.instances.spheres.len()
            && self.instances.sticks.len() == other.instances.sticks.len()
            && self.instances.outlines.len() == other.instances.outlines.len();
        if !same_layout {
            // Cartoon/SES topology can change between scientific snapshots.
            // Preserve discrete mesh playback in that case instead of trying
            // to interpolate unrelated vertices (or crashing an animation).
            if self.instances.spheres.is_empty()
                && self.instances.sticks.is_empty()
                && other.instances.spheres.is_empty()
                && other.instances.sticks.is_empty()
            {
                return self.clone();
            }
            logger.error("Display interpolation requires matching render geometry layouts");
            panic!("Display interpolation requires matching render geometry layouts");
        }
        fn lerp<const N: usize>(a: [f32; N], b: [f32; N], t: f32) -> [f32; N] {
            std::array::from_fn(|i| a[i] * (1.0 - t) + b[i] * t)
        }
        fn groups(a: &mut InstanceGroups, b: &InstanceGroups, t: f32) {
            for (a, b) in a.spheres.iter_mut().zip(&b.spheres) {
                a.position = lerp(a.position, b.position, t);
                a.radius = a.radius * (1.0 - t) + b.radius * t;
                a.color = lerp(a.color, b.color, t);
                a.material = lerp(a.material, b.material, t);
            }
            for (a, b) in a.sticks.iter_mut().zip(&b.sticks) {
                a.start = lerp(a.start, b.start, t);
                a.end = lerp(a.end, b.end, t);
                a.radius = a.radius * (1.0 - t) + b.radius * t;
                a.color = lerp(a.color, b.color, t);
                a.material = lerp(a.material, b.material, t);
            }
            // Outlines follow the same interpolated instance geometry.
            for outline in &mut a.outlines {
                outline.spheres.clone_from(&a.spheres);
                outline.sticks.clone_from(&a.sticks);
            }
        }
        let mut display = self.clone();
        for (a, b) in display.mesh.vertices.iter_mut().zip(&other.mesh.vertices) {
            *a = a.lerp(*b, t);
        }
        for (a, b) in display.mesh.normals.iter_mut().zip(&other.mesh.normals) {
            *a = a.lerp(*b, t).normalize_or_zero();
        }
        if let (Some(a), Some(b)) = (&mut display.mesh.colors, &other.mesh.colors) {
            for (a, b) in a.iter_mut().zip(b) {
                *a = a.lerp(*b, t);
            }
        }
        groups(&mut display.instances, &other.instances, t);
        display.center = self.center.lerp(other.center, t);
        display
    }
}
