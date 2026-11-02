//! Backbone/surface geometry evaluated from borrowed CK protein values.
//! Temporary DSSP inputs and surface points are discarded after projection.
use super::input::{
    ProteinAtomInput as RawSurfacePoint, ProteinInput, ProteinResidueInput as RawProteinResidue,
};
use super::{DisplayError, center};
use crate::parser::dssp::SecondaryStructureCalculator;
use crate::parser::utils::{Residue, RibbonResidueInfo, SecondaryStructure};
use crate::shapes::{SphereInstance, Stick};
use crate::styles::{Cartoon, Representation, Surface};
use crate::surface::protein_templates::{self, ResiduePosition};
use crate::surface::radii::default_radius;
use crate::surface::{ses_surface_geometry, sharp_edge_patches};
use crate::utils::{InstanceGroups, MeshData};
use cosmolkit::{self as ck, BioCoordinateFormat};
use glam::{Quat, Vec3, Vec4};
use std::collections::BTreeSet;

struct SurfacePoint {
    id: ck::BioAtomId,
    position: Vec3,
    radius: f32,
}

pub(super) fn project_residues(
    protein: &ProteinInput,
    ids: &[ck::BioResidueId],
    style: Representation,
) -> Result<(MeshData, InstanceGroups, Vec3), DisplayError> {
    if let Some(id) = ids
        .iter()
        .find(|id| id.index() >= protein.chains.iter().map(Vec::len).sum::<usize>())
    {
        return Err(DisplayError::InvalidEntity {
            kind: "residue",
            index: id.index(),
        });
    }
    match style {
        Representation::Cartoon(style) => {
            let (mesh, center) = cartoon(protein, ids, &style)?;
            Ok((mesh, InstanceGroups::default(), center))
        }
        other => Err(DisplayError::IncompatibleStyle {
            style: other.name(),
            selection: "protein residues (select atoms explicitly for atom/surface styles)",
        }),
    }
}

pub(super) fn project_atoms(
    protein: &ProteinInput,
    ids: &[ck::BioAtomId],
    style: Representation,
) -> Result<(MeshData, InstanceGroups, Vec3), DisplayError> {
    if let Some(id) = ids.iter().find(|id| {
        id.index()
            >= protein
                .chains
                .iter()
                .flatten()
                .map(|r| r.atoms.len())
                .sum::<usize>()
    }) {
        return Err(DisplayError::InvalidEntity {
            kind: "atom",
            index: id.index(),
        });
    }
    match style {
        Representation::Surface(style) => {
            let (mesh, center) = surface(protein, ids, &style)?;
            Ok((mesh, InstanceGroups::default(), center))
        }
        Representation::Sphere(style) => {
            if !matches!(style.options.coordinates, crate::styles::Coordinates::Auto) {
                return Err(DisplayError::IncompatibleStyle {
                    style: "molecular coordinate selection",
                    selection: "protein atoms, whose coordinates belong to BIO models",
                });
            }
            let selected: BTreeSet<_> = ids.iter().copied().collect();
            let mut groups = InstanceGroups::default();
            let mut positions = Vec::new();
            for atom in protein
                .chains
                .iter()
                .flatten()
                .flat_map(|r| &r.atoms)
                .filter(|a| selected.contains(&a.id))
            {
                let p = atom.position;
                positions.push(p);
                let element = ck::Element::from_atomic_number(atom.atomic_number)
                    .ok_or(DisplayError::InvalidElement(atom.atomic_number))?;
                let color = style
                    .options
                    .material
                    .color
                    .unwrap_or_else(|| super::molecular::element_color(&element));
                groups.spheres.push(SphereInstance::new(
                    p.to_array(),
                    super::molecular::vdw_radius(
                        &ck::Element::from_atomic_number(atom.atomic_number)
                            .ok_or(DisplayError::InvalidElement(atom.atomic_number))?,
                    ) * style.options.radius_scale,
                    [color.x, color.y, color.z, style.options.material.opacity],
                    [
                        style.options.material.roughness,
                        style.options.material.metallic,
                    ],
                ));
            }
            if style.options.outline.enabled && !groups.spheres.is_empty() {
                groups.outlines.push(crate::utils::OutlineInstanceGroup {
                    settings: style.options.outline,
                    spheres: groups.spheres.clone(),
                    sticks: Vec::new(),
                });
            }
            Ok((MeshData::default(), groups, center(positions.into_iter())))
        }
        Representation::Cartoon(_) => Err(DisplayError::IncompatibleStyle {
            style: "Cartoon",
            selection: "protein atoms (select residues explicitly)",
        }),
        Representation::BallAndStick(_) => Err(DisplayError::IncompatibleStyle {
            style: "BallAndStick",
            selection: "protein atoms without a mapped chemical bond graph",
        }),
        Representation::Stick(_) => Err(DisplayError::IncompatibleStyle {
            style: "Stick",
            selection: "protein atoms without a mapped chemical bond graph",
        }),
    }
}

fn cartoon(
    protein: &ProteinInput,
    ids: &[ck::BioResidueId],
    style: &Cartoon,
) -> Result<(MeshData, Vec3), DisplayError> {
    let selected: BTreeSet<_> = ids.iter().copied().collect();
    if selected.is_empty() {
        return Ok((MeshData::default(), Vec3::ZERO));
    }
    let mut mesh = MeshData::default();
    let mut centers = Vec::new();
    for mut chain in protein.chains.clone() {
        // A residue representation chooses one coherent backbone altloc using
        // the existing occupancy/B-factor policy. Atom selections are not changed.
        select_best_alt_locs(&mut chain);
        let palette = (style.rainbow && style.material.color.is_none())
            .then(|| chimerax_rainbow_residue_colors(chain.len(), style.material.opacity));
        let mut backbones = Vec::new();
        for raw in &chain {
            let p = |name: &str| {
                raw.atoms
                    .iter()
                    .find(|a| a.name == name)
                    .map(|a| a.position)
            };
            let backbone = match (p("CA"), p("C"), p("N"), p("O")) {
                (Some(ca), Some(c), Some(n), Some(o)) => Some(Residue {
                    residue_type: raw.residue_type,
                    ca,
                    c,
                    n,
                    o,
                    h: p("H"),
                    sns: raw.seq_id.map(|(n, _)| n.max(0) as usize).unwrap_or(0),
                    ss: None,
                }),
                _ if selected.contains(&raw.id) => {
                    return Err(DisplayError::MissingBackbone(raw.id.value()));
                }
                _ => None,
            };
            backbones.push(backbone);
        }
        let mut start = 0;
        while start < backbones.len() {
            if backbones[start].is_none() {
                start += 1;
                continue;
            }
            let end = (start..backbones.len())
                .find(|&i| backbones[i].is_none())
                .unwrap_or(backbones.len());
            let full: Vec<Residue> = backbones[start..end]
                .iter()
                .map(|r| r.as_ref().unwrap().clone())
                .collect();
            // DSSP uses complete available chain context, not only selected rows.
            let info = SecondaryStructureCalculator::new().compute_ribbon_info(&full);
            let mut run = 0;
            while run < full.len() {
                if !selected.contains(&chain[start + run].id) {
                    run += 1;
                    continue;
                }
                let stop = (run..full.len())
                    .find(|&i| !selected.contains(&chain[start + i].id))
                    .unwrap_or(full.len());
                let residues: Vec<_> = full[run..stop].iter().collect();
                centers.extend(residues.iter().map(|r| r.ca));
                let colors = palette
                    .as_ref()
                    .map(|p| p[start + run..start + stop].to_vec());
                if residues.len() >= 2 {
                    mesh.append(&build_chimerax_ribbon_mesh(
                        &residues,
                        &info[run..stop],
                        colors.as_deref(),
                    ));
                }
                run = stop;
            }
            start = end;
        }
    }
    apply_material(&mut mesh, style.material);
    Ok((mesh, center(centers.into_iter())))
}

fn surface(
    protein: &ProteinInput,
    ids: &[ck::BioAtomId],
    style: &Surface,
) -> Result<(MeshData, Vec3), DisplayError> {
    if ids.is_empty() {
        return Ok((MeshData::default(), Vec3::ZERO));
    }
    let selected: BTreeSet<_> = ids.iter().copied().collect();
    let chains = protein.chains.clone();
    let has_hydrogens = chains
        .iter()
        .flatten()
        .flat_map(|r| &r.atoms)
        .any(|a| a.atomic_number == 1);
    let points: Vec<_> = chains
        .iter()
        .flat_map(|chain| surface_atoms(chain, has_hydrogens, protein.input_format))
        .filter(|point| selected.contains(&point.id))
        .collect();
    let positions: Vec<_> = points.iter().map(|p| p.position).collect();
    let radii: Vec<_> = points.iter().map(|p| p.radius).collect();
    let mut geometry = ses_surface_geometry(
        &positions,
        &radii,
        style.probe_radius,
        style.grid_spacing,
        style.solvent_accessible,
    )?;
    if style.sharp_boundaries {
        geometry = sharp_edge_patches(
            geometry,
            &positions,
            &radii,
            style.probe_radius,
            style.grid_spacing,
            1,
        );
    }
    let mut mesh = MeshData {
        vertices: geometry.vertices,
        normals: geometry.normals,
        indices: geometry.triangles.into_iter().flatten().collect(),
        ..Default::default()
    };
    apply_material(&mut mesh, style.material);
    Ok((mesh, center(positions.into_iter())))
}

fn apply_material(mesh: &mut MeshData, material: crate::utils::Material) {
    if !material.visible {
        *mesh = MeshData::default();
        return;
    }
    if mesh.colors.is_none() {
        let color = material.color.unwrap_or(Vec3::ONE).extend(material.opacity);
        mesh.colors = Some(vec![color; mesh.vertices.len()]);
    }
    mesh.material_params = Some(vec![
        [material.roughness, material.metallic];
        mesh.vertices.len()
    ]);
    mesh.is_wireframe = material.wireframe;
}

fn select_best_alt_locs(residues: &mut [RawProteinResidue]) {
    for residue in residues {
        let mut atom_groups: Vec<Vec<RawSurfacePoint>> = Vec::new();
        for atom in residue.atoms.drain(..) {
            if let Some(group) = atom_groups
                .iter_mut()
                .find(|group| group[0].name == atom.name)
            {
                group.push(atom);
            } else {
                atom_groups.push(vec![atom]);
            }
        }

        let alt_locs = atom_groups.iter().find_map(|group| {
            let labels: BTreeSet<u8> = group.iter().filter_map(|atom| atom.alt_loc).collect();
            (!labels.is_empty()).then_some(labels)
        });
        let best_alt_loc = alt_locs
            .as_ref()
            .and_then(|labels| best_alt_loc(&atom_groups, labels));

        residue.atoms = atom_groups
            .into_iter()
            .map(|group| {
                let index = best_alt_loc
                    .and_then(|label| group.iter().position(|atom| atom.alt_loc == Some(label)))
                    .or_else(|| group.iter().rposition(|atom| atom.alt_loc.is_some()))
                    .unwrap_or(0);
                group.into_iter().nth(index).expect("atom index is valid")
            })
            .collect();
    }
}

fn best_alt_loc(atom_groups: &[Vec<RawSurfacePoint>], labels: &BTreeSet<u8>) -> Option<u8> {
    let mut best = None;
    let mut best_occupancy = 0.0f32;
    let mut best_b_factor = 0.0f32;

    for &label in labels {
        let mut occurrences = 0usize;
        let mut occupancy = 0.0f32;
        let mut b_factor = 0.0f32;
        for group in atom_groups {
            if !labels
                .iter()
                .all(|candidate| group.iter().any(|atom| atom.alt_loc == Some(*candidate)))
            {
                continue;
            }
            let atom = group
                .iter()
                .find(|atom| atom.alt_loc == Some(label))
                .expect("complete alternate-location group");
            occurrences += 1;
            occupancy += atom.occupancy as f32;
            b_factor += atom.b_factor as f32;
        }
        if occurrences == 0 {
            continue;
        }
        occupancy /= occurrences as f32;
        b_factor /= occurrences as f32;
        if best.is_none()
            || occupancy > best_occupancy
            || (occupancy == best_occupancy && b_factor < best_b_factor)
        {
            best = Some(label);
            best_occupancy = occupancy;
            best_b_factor = b_factor;
        }
    }

    best
}

fn surface_atoms(
    residues: &[RawProteinResidue],
    structure_has_hydrogens: bool,
    input_format: BioCoordinateFormat,
) -> Vec<SurfacePoint> {
    let mut result = Vec::new();
    for (residue_index, residue) in residues.iter().enumerate() {
        let chain_start = residue_index == 0;
        let chain_end = residue_index + 1 == residues.len();
        let position = if chain_start {
            ResiduePosition::Start
        } else if chain_end {
            ResiduePosition::End
        } else {
            ResiduePosition::Middle
        };
        let topology_name = if residue.name == "HIS" {
            "HIP"
        } else {
            residue.name.as_str()
        };
        let present = |name: &str| residue.atoms.iter().any(|atom| atom.name == name);
        let bonds = protein_templates::bonds(position, topology_name);

        for atom in &residue.atoms {
            let mut bond_count = 0;
            let mut has_hydrogen_neighbor = false;
            for &(first, second) in bonds {
                let neighbor = if first == atom.name && present(second) {
                    Some(second)
                } else if second == atom.name && present(first) {
                    Some(first)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor {
                    bond_count += 1;
                    has_hydrogen_neighbor |= residue
                        .atoms
                        .iter()
                        .find(|candidate| candidate.name == neighbor)
                        .is_some_and(|candidate| candidate.atomic_number == 1);
                }
            }

            if atom.name == "N"
                && !chain_start
                && residues_are_polymer_bonded(&residues[residue_index - 1], residue, input_format)
                && residues[residue_index - 1]
                    .atoms
                    .iter()
                    .any(|candidate| candidate.name == "C")
            {
                bond_count += 1;
            }
            if atom.name == "C"
                && !chain_end
                && residues_are_polymer_bonded(residue, &residues[residue_index + 1], input_format)
                && residues[residue_index + 1]
                    .atoms
                    .iter()
                    .any(|candidate| candidate.name == "N")
            {
                bond_count += 1;
            }
            if atom.name == "C"
                && present("OXT")
                && !bonds
                    .iter()
                    .any(|&(first, second)| first == "C" && second == "OXT")
            {
                bond_count += 1;
            }
            if atom.name == "OXT"
                && present("C")
                && !bonds
                    .iter()
                    .any(|&(first, second)| first == "C" && second == "OXT")
            {
                bond_count += 1;
            }

            let idatm_type =
                protein_templates::idatm_type(&residue.name, &atom.name, chain_start, present);
            result.push(SurfacePoint {
                id: atom.id,
                position: atom.position,
                radius: default_radius(
                    atom.atomic_number,
                    idatm_type,
                    bond_count,
                    has_hydrogen_neighbor,
                    structure_has_hydrogens,
                ),
            });
        }
    }
    result
}

fn residues_are_polymer_bonded(
    previous: &RawProteinResidue,
    next: &RawProteinResidue,
    input_format: BioCoordinateFormat,
) -> bool {
    if input_format == BioCoordinateFormat::Mmcif {
        return matches!(
            (previous.label_seq_id, next.label_seq_id),
            (Some(previous_id), Some(next_id)) if next_id == previous_id + 1
        );
    }

    if matches!(
        (previous.seq_id, next.seq_id),
        (Some((previous_number, _)), Some((next_number, _)))
            if (next_number - previous_number).abs() < 2
    ) {
        return true;
    }

    let Some(carbon) = previous
        .atoms
        .iter()
        .find(|atom| atom.name == "C")
        .map(|atom| atom.position)
    else {
        return false;
    };
    let Some(nitrogen) = next
        .atoms
        .iter()
        .find(|atom| atom.name == "N")
        .map(|atom| atom.position)
    else {
        return false;
    };

    // ChimeraX Element::bond_length(C, N) is 0.68 + 0.68 Angstrom.
    carbon.distance_squared(nitrogen) < 3.0625 * 1.36 * 1.36
}

const CHIMERAX_RIBBON_DIVISIONS: usize = 20;
const BACKBONE_GAP_CA_DISTANCE: f32 = 4.7;
const BACKBONE_GAP_PEPTIDE_DISTANCE: f32 = 2.2;
const BACKBONE_GAP_DASH_RADIUS: f32 = 0.06;
const BACKBONE_GAP_DASH_LENGTH: f32 = 0.45;
const BACKBONE_GAP_DASH_SPACING: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RibbonClass {
    Coil,
    SheetStart,
    SheetMiddle,
    SheetEnd,
    HelixStart,
    HelixMiddle,
    HelixEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RibbonRef {
    Coil,
    Sheet,
    SheetArrow,
    Helix,
    HelixArrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RibbonRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct RibbonXSection {
    kind: RibbonRef,
}

struct ChimeraXSectionManager;

impl ChimeraXSectionManager {
    fn assign(
        &self,
        rc0: RibbonClass,
        rc1: RibbonClass,
        rc2: RibbonClass,
    ) -> (RibbonRef, RibbonRef) {
        match rc1 {
            RibbonClass::SheetMiddle => (RibbonRef::Sheet, RibbonRef::Sheet),
            RibbonClass::HelixMiddle => (RibbonRef::Helix, RibbonRef::Helix),
            RibbonClass::Coil => (RibbonRef::Coil, RibbonRef::Coil),
            RibbonClass::SheetStart => match (rc0, rc2) {
                (
                    RibbonClass::Coil | RibbonClass::HelixEnd | RibbonClass::SheetEnd,
                    RibbonClass::SheetMiddle | RibbonClass::SheetEnd,
                ) => (RibbonRef::Coil, RibbonRef::Sheet),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::SheetEnd => match (rc0, rc2) {
                (
                    RibbonClass::SheetStart | RibbonClass::SheetMiddle,
                    RibbonClass::Coil | RibbonClass::HelixStart | RibbonClass::SheetStart,
                ) => (RibbonRef::SheetArrow, RibbonRef::Coil),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::HelixStart => match (rc0, rc2) {
                (
                    RibbonClass::Coil | RibbonClass::HelixEnd | RibbonClass::SheetEnd,
                    RibbonClass::HelixMiddle | RibbonClass::HelixEnd,
                ) => (RibbonRef::Coil, RibbonRef::Helix),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::HelixEnd => match (rc0, rc2) {
                (
                    RibbonClass::HelixStart | RibbonClass::HelixMiddle,
                    RibbonClass::Coil | RibbonClass::HelixStart | RibbonClass::SheetStart,
                ) => (RibbonRef::HelixArrow, RibbonRef::Coil),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
        }
    }

    fn xsection(&self, kind: RibbonRef) -> RibbonXSection {
        RibbonXSection { kind }
    }
}

fn build_chimerax_ribbon_mesh(
    residues: &[&Residue],
    ribbon_info: &[RibbonResidueInfo],
    residue_colors: Option<&[Vec4]>,
) -> MeshData {
    if residues.len() < 2 {
        return MeshData::default();
    }

    let mut mesh = MeshData::default();
    for range in contiguous_backbone_ranges(residues) {
        if range.end - range.start < 2 {
            continue;
        }

        let segment_colors = residue_colors.map(|colors| &colors[range.start..range.end]);
        mesh.append(&build_chimerax_ribbon_segment_mesh(
            &residues[range.start..range.end],
            &ribbon_info[range.start..range.end],
            segment_colors,
        ));
    }
    mesh.append(&build_backbone_gap_dashes(residues, residue_colors));
    mesh
}

fn build_chimerax_ribbon_segment_mesh(
    residues: &[&Residue],
    ribbon_info: &[RibbonResidueInfo],
    residue_colors: Option<&[Vec4]>,
) -> MeshData {
    let mut centers: Vec<Vec3> = residues.iter().map(|residue| residue.ca).collect();
    let mut guides: Vec<Vec3> = residues
        .iter()
        .map(|residue| residue.o - residue.c)
        .collect();
    let (res_class, _helix_ranges, sheet_ranges, display_ranges) = ribbon_ranges(ribbon_info);
    smooth_strands(&mut centers, &mut guides, &sheet_ranges);

    let coeffs = natural_cubic_spline_coefficients(&centers);
    let control_tangents = spline_control_tangents(&coeffs);
    let control_normals = path_plane_normals(&centers, &control_tangents);
    let smooth_twist = ribbon_smooth_twist(&res_class);
    let path = spline_path(
        &coeffs,
        &control_normals,
        &ribbon_flip_normals(ribbon_info),
        &smooth_twist,
        CHIMERAX_RIBBON_DIVISIONS,
    );

    let manager = ChimeraXSectionManager;
    let (xs_front, xs_back) = ribbon_crosssections(&res_class, &manager);
    ribbon_extrusions(
        &path.centers,
        &path.tangents,
        &path.normals,
        &display_ranges,
        residues.len(),
        &xs_front,
        &xs_back,
        residue_colors,
        &manager,
    )
}

fn contiguous_backbone_ranges(residues: &[&Residue]) -> Vec<RibbonRange> {
    if residues.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0;
    for i in 1..residues.len() {
        if !residue_backbone_connected(residues[i - 1], residues[i]) {
            ranges.push(RibbonRange { start, end: i });
            start = i;
        }
    }
    ranges.push(RibbonRange {
        start,
        end: residues.len(),
    });
    ranges
}

fn residue_backbone_connected(previous: &Residue, next: &Residue) -> bool {
    if previous.ca.distance(next.ca) > BACKBONE_GAP_CA_DISTANCE {
        return false;
    }

    let has_peptide_atoms = previous.c.length_squared() > 0.0 && next.n.length_squared() > 0.0;
    if has_peptide_atoms && previous.c.distance(next.n) > BACKBONE_GAP_PEPTIDE_DISTANCE {
        return false;
    }

    true
}

fn build_backbone_gap_dashes(residues: &[&Residue], residue_colors: Option<&[Vec4]>) -> MeshData {
    let mut mesh = MeshData::default();
    for i in 1..residues.len() {
        if residue_backbone_connected(residues[i - 1], residues[i]) {
            continue;
        }

        let color = residue_colors
            .and_then(|colors| colors.get(i - 1).zip(colors.get(i)))
            .map(|(previous, next)| (*previous + *next) * 0.5);
        mesh.append(&dashed_backbone_gap_mesh(
            residues[i - 1].ca,
            residues[i].ca,
            color,
        ));
    }
    mesh
}

fn dashed_backbone_gap_mesh(start: Vec3, end: Vec3, color: Option<Vec4>) -> MeshData {
    let delta = end - start;
    let length = delta.length();
    if length <= f32::EPSILON {
        return MeshData::default();
    }

    let direction = delta / length;
    let period = BACKBONE_GAP_DASH_LENGTH + BACKBONE_GAP_DASH_SPACING;
    let mut offset = 0.0;
    let mut mesh = MeshData::default();

    while offset < length {
        let dash_start = start + direction * offset;
        let dash_end = start + direction * (offset + BACKBONE_GAP_DASH_LENGTH).min(length);
        if dash_start.distance_squared(dash_end) > f32::EPSILON {
            let mut dash = Stick::new(
                dash_start.to_array(),
                dash_end.to_array(),
                BACKBONE_GAP_DASH_RADIUS,
            )
            .to_mesh(1.0);
            if let Some(color) = color {
                dash.colors = Some(vec![color; dash.vertices.len()]);
            } else {
                dash.colors = None;
            }
            dash.material_params = None;
            mesh.append(&dash);
        }
        offset += period;
    }

    mesh
}

fn chimerax_rainbow_residue_colors(residue_count: usize, opacity: f32) -> Vec<Vec4> {
    // BEGIN CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: _builtin_colormaps
    // ChimeraX✔️✔️: BuiltinColormaps["rainbow"] is blue, cyan, green, yellow, red.
    // ChimeraX✔️✔️: Colormap(None, colors) assigns evenly spaced values from 0.0 to 1.0.
    // END CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: _builtin_colormaps
    //
    // BEGIN CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: Colormap.interpolated_rgba8
    // ChimeraX✔️✔️: interpolated_rgba linearly interpolates in RGB/RGBA space, then rgba8 scales by 255.
    // END CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: Colormap.interpolated_rgba8
    const STOPS: [Vec3; 5] = [
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ];

    if residue_count == 0 {
        return Vec::new();
    }

    (0..residue_count)
        .map(|i| {
            let value = if residue_count == 1 {
                0.0
            } else {
                i as f32 / (residue_count - 1) as f32
            };
            let scaled = value * (STOPS.len() - 1) as f32;
            let lower = scaled.floor() as usize;
            let upper = lower.min(STOPS.len() - 1);
            let next = (lower + 1).min(STOPS.len() - 1);
            let t = scaled - lower as f32;
            let color = STOPS[upper] * (1.0 - t) + STOPS[next] * t;
            Vec4::new(
                chimerax_rgba8_channel_to_float(color.x),
                chimerax_rgba8_channel_to_float(color.y),
                chimerax_rgba8_channel_to_float(color.z),
                opacity,
            )
        })
        .collect()
}

fn chimerax_rgba8_channel_to_float(channel: f32) -> f32 {
    (channel.clamp(0.0, 1.0) * 255.0).trunc() / 255.0
}

fn ribbon_ranges(
    ribbon_info: &[RibbonResidueInfo],
) -> (
    Vec<RibbonClass>,
    Vec<RibbonRange>,
    Vec<RibbonRange>,
    Vec<RibbonRange>,
) {
    let mut classes = Vec::with_capacity(ribbon_info.len());
    let mut helix_ranges: Vec<RibbonRange> = Vec::new();
    let mut sheet_ranges: Vec<RibbonRange> = Vec::new();
    let mut was_sheet = false;
    let mut was_helix = false;
    let mut last_sheet_id = None;
    let mut last_helix_id = None;

    for (i, info) in ribbon_info.iter().enumerate() {
        let mut am_sheet = false;
        let mut am_helix = false;
        let class = if info.ss == SecondaryStructure::Sheet {
            am_sheet = true;
            if was_sheet && info.sheet_id == last_sheet_id {
                RibbonClass::SheetMiddle
            } else {
                if was_sheet {
                    end_strand(&mut classes, &mut sheet_ranges, i);
                }
                sheet_ranges.push(RibbonRange {
                    start: i,
                    end: usize::MAX,
                });
                RibbonClass::SheetStart
            }
        } else if info.ss == SecondaryStructure::Helix {
            am_helix = true;
            if was_helix && info.helix_id == last_helix_id {
                RibbonClass::HelixMiddle
            } else {
                if was_helix {
                    end_helix(&mut classes, &mut helix_ranges, i);
                }
                helix_ranges.push(RibbonRange {
                    start: i,
                    end: usize::MAX,
                });
                RibbonClass::HelixStart
            }
        } else {
            RibbonClass::Coil
        };

        if was_sheet && !am_sheet {
            end_strand(&mut classes, &mut sheet_ranges, i);
        } else if was_helix && !am_helix {
            end_helix(&mut classes, &mut helix_ranges, i);
        }

        classes.push(class);
        was_sheet = am_sheet;
        was_helix = am_helix;
        last_sheet_id = info.sheet_id;
        last_helix_id = info.helix_id;
    }

    if was_sheet {
        end_strand(&mut classes, &mut sheet_ranges, ribbon_info.len());
    } else if was_helix {
        end_helix(&mut classes, &mut helix_ranges, ribbon_info.len());
    }

    let display_ranges = if ribbon_info.is_empty() {
        Vec::new()
    } else {
        vec![RibbonRange {
            start: 0,
            end: ribbon_info.len(),
        }]
    };

    (classes, helix_ranges, sheet_ranges, display_ranges)
}

fn end_strand(classes: &mut [RibbonClass], ranges: &mut Vec<RibbonRange>, end: usize) {
    if classes.last() == Some(&RibbonClass::SheetStart) {
        if let Some(last) = classes.last_mut() {
            *last = RibbonClass::Coil;
        }
        ranges.pop();
    } else if let Some(last) = classes.last_mut() {
        *last = RibbonClass::SheetEnd;
        if let Some(range) = ranges.last_mut() {
            range.end = end;
        }
    }
}

fn end_helix(classes: &mut [RibbonClass], ranges: &mut Vec<RibbonRange>, end: usize) {
    if classes.last() == Some(&RibbonClass::HelixStart) {
        if let Some(last) = classes.last_mut() {
            *last = RibbonClass::Coil;
        }
        ranges.pop();
    } else if let Some(last) = classes.last_mut() {
        *last = RibbonClass::HelixEnd;
        if let Some(range) = ranges.last_mut() {
            range.end = end;
        }
    }
}

fn ribbon_crosssections(
    classes: &[RibbonClass],
    manager: &ChimeraXSectionManager,
) -> (Vec<RibbonXSection>, Vec<RibbonXSection>) {
    let mut front = Vec::with_capacity(classes.len());
    let mut back = Vec::with_capacity(classes.len());
    let mut rc0 = RibbonClass::Coil;

    for i in 0..classes.len() {
        let rc1 = classes[i];
        let rc2 = classes.get(i + 1).copied().unwrap_or(RibbonClass::Coil);
        let (f, b) = manager.assign(rc0, rc1, rc2);
        front.push(manager.xsection(f));
        back.push(manager.xsection(b));
        rc0 = rc1;
    }

    (front, back)
}

fn ribbon_smooth_twist(classes: &[RibbonClass]) -> Vec<bool> {
    let mut twist = Vec::with_capacity(classes.len());
    for i in 0..classes.len() {
        let rc0 = classes[i];
        let rc1 = classes.get(i + 1).copied().unwrap_or(RibbonClass::Coil);
        twist.push(smooth_twist_between(rc0, rc1));
    }
    if let Some(last) = twist.last_mut() {
        *last = false;
    }
    twist
}

fn smooth_twist_between(rc0: RibbonClass, rc1: RibbonClass) -> bool {
    rc0 == rc1
        || (is_sheet_class(rc0) && is_sheet_class(rc1))
        || (is_helix_class(rc0) && is_helix_class(rc1))
        || matches!(rc0, RibbonClass::HelixEnd | RibbonClass::SheetEnd)
        || matches!(rc1, RibbonClass::HelixStart | RibbonClass::SheetStart)
}

fn is_sheet_class(class: RibbonClass) -> bool {
    matches!(
        class,
        RibbonClass::SheetStart | RibbonClass::SheetMiddle | RibbonClass::SheetEnd
    )
}

fn is_helix_class(class: RibbonClass) -> bool {
    matches!(
        class,
        RibbonClass::HelixStart | RibbonClass::HelixMiddle | RibbonClass::HelixEnd
    )
}

fn ribbon_flip_normals(ribbon_info: &[RibbonResidueInfo]) -> Vec<bool> {
    let last = ribbon_info.len().saturating_sub(1);
    ribbon_info
        .iter()
        .enumerate()
        .map(|(i, info)| {
            !(info.ss == SecondaryStructure::Helix
                && (i == last || ribbon_info[i + 1].ss == SecondaryStructure::Helix))
        })
        .collect()
}

fn smooth_strands(centers: &mut [Vec3], guides: &mut [Vec3], sheet_ranges: &[RibbonRange]) {
    for range in sheet_ranges {
        smooth_strand(centers, guides, range.start, range.end);
    }
}

fn smooth_strand(centers: &mut [Vec3], guides: &mut [Vec3], start: usize, end: usize) {
    if end <= start || end - start <= 2 {
        return;
    }

    let original = centers[start..end].to_vec();
    let len = original.len();
    if len < 3 {
        return;
    }

    let mut ideal = vec![Vec3::ZERO; len];
    for i in 1..(len - 1) {
        ideal[i] = (original[i] * 2.0 + original[i - 1] + original[i + 1]) * 0.25;
    }
    if len == 3 {
        ideal[0] = original[0] - 0.99 * (ideal[1] - original[1]);
        ideal[len - 1] = original[len - 1] - 0.99 * (ideal[len - 2] - original[len - 2]);
    } else {
        ideal[0] = original[0] - (ideal[1] - original[1]);
        ideal[len - 1] = original[len - 1] - (ideal[len - 2] - original[len - 2]);
    }

    for i in 0..len {
        let delta_guide = guides[start + i];
        centers[start + i] = ideal[i];
        guides[start + i] = delta_guide;
    }
}

#[derive(Clone)]
struct RibbonPath {
    centers: Vec<Vec3>,
    tangents: Vec<Vec3>,
    normals: Vec<Vec3>,
}

fn natural_cubic_spline_coefficients(points: &[Vec3]) -> Vec<[Vec3; 4]> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }

    let mut extended = Vec::with_capacity(n + 2);
    extended.push(points[0] - (points[1] - points[0]));
    extended.extend_from_slice(points);
    extended.push(points[n - 1] + (points[n - 1] - points[n - 2]));

    let ne = extended.len();
    let mut deriv = vec![Vec3::ZERO; ne];
    for axis in 0..3 {
        let values: Vec<f32> = extended.iter().map(|p| p[axis]).collect();
        let mut a = vec![1.0f32; ne];
        let mut b = vec![4.0f32; ne];
        let c = vec![1.0f32; ne];
        let mut d = vec![0.0f32; ne];
        b[0] = 2.0;
        b[ne - 1] = 2.0;
        d[0] = values[1] - values[0];
        for i in 1..(ne - 1) {
            d[i] = 3.0 * (values[i + 1] - values[i - 1]);
        }
        d[ne - 1] = 3.0 * (values[ne - 1] - values[ne - 2]);
        let solved = tridiagonal(&mut a, &mut b, &c, &mut d);
        for i in 0..ne {
            deriv[i][axis] = solved[i];
        }
    }

    let mut coeffs = Vec::with_capacity(n - 1);
    for i in 1..(ne - 2) {
        let delta = extended[i + 1] - extended[i];
        coeffs.push([
            extended[i],
            deriv[i],
            delta * 3.0 - deriv[i] * 2.0 - deriv[i + 1],
            delta * -2.0 + deriv[i] + deriv[i + 1],
        ]);
    }
    coeffs
}

fn tridiagonal(a: &mut [f32], b: &mut [f32], c: &[f32], d: &mut [f32]) -> Vec<f32> {
    let n = a.len();
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        d[i] -= m * d[i - 1];
    }

    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1] / b[n - 1];
    for i in (0..(n - 1)).rev() {
        x[i] = (d[i] - c[i] * x[i + 1]) / b[i];
    }
    x
}

fn spline_control_tangents(coeffs: &[[Vec3; 4]]) -> Vec<Vec3> {
    if coeffs.is_empty() {
        return Vec::new();
    }
    let mut tangents: Vec<Vec3> = coeffs.iter().map(|c| c[1].normalize_or_zero()).collect();
    let last = coeffs.last().unwrap();
    tangents.push((last[1] + last[2] * 2.0 + last[3] * 3.0).normalize_or_zero());
    tangents
}

fn spline_path(
    coeffs: &[[Vec3; 4]],
    normals: &[Vec3],
    flip_normals: &[bool],
    twist: &[bool],
    divisions: usize,
) -> RibbonPath {
    let mut centers = Vec::new();
    let mut tangents = Vec::new();
    let mut path_normals = Vec::new();
    if coeffs.is_empty() {
        return RibbonPath {
            centers,
            tangents,
            normals: path_normals,
        };
    }

    let lead_n = divisions / 2;
    let (lead_c, lead_t) = cubic_path_points(&coeffs[0], -0.3, 0.0, lead_n + 1);
    let lead_nrm = parallel_transport_normals(&lead_t, normals[0], true);
    append_without_last(&mut centers, &lead_c);
    append_without_last(&mut tangents, &lead_t);
    append_without_last(&mut path_normals, &lead_nrm);

    let mut end_normal = normals[0];
    for seg in 0..coeffs.len() {
        let (seg_c, seg_t) = cubic_path_points(&coeffs[seg], 0.0, 1.0, divisions + 1);
        let start_normal = if seg == 0 { normals[0] } else { end_normal };
        let mut seg_n = parallel_transport_normals(&seg_t, start_normal, false);
        end_normal = normals[seg + 1];
        if twist.get(seg).copied().unwrap_or(false) {
            if flip_normals.get(seg).copied().unwrap_or(false)
                && need_normal_flip(*seg_n.last().unwrap(), end_normal, *seg_t.last().unwrap())
            {
                end_normal = -end_normal;
            }
            smooth_twist(&seg_t, &mut seg_n, end_normal);
        }
        append_without_last(&mut centers, &seg_c);
        append_without_last(&mut tangents, &seg_t);
        append_without_last(&mut path_normals, &seg_n);
    }

    let trail_n = (divisions + 1) / 2;
    let (trail_c, trail_t) = cubic_path_points(coeffs.last().unwrap(), 1.0, 1.3, trail_n);
    let trail_nrm = parallel_transport_normals(&trail_t, end_normal, false);
    centers.extend(trail_c);
    tangents.extend(trail_t);
    path_normals.extend(trail_nrm);

    RibbonPath {
        centers,
        tangents,
        normals: path_normals,
    }
}

fn append_without_last<T: Copy>(target: &mut Vec<T>, values: &[T]) {
    if !values.is_empty() {
        target.extend_from_slice(&values[..values.len() - 1]);
    }
}

fn cubic_path_points(coeff: &[Vec3; 4], tmin: f32, tmax: f32, n: usize) -> (Vec<Vec3>, Vec<Vec3>) {
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut coords = Vec::with_capacity(n);
    let mut tangents = Vec::with_capacity(n);
    let step = if n > 1 {
        (tmax - tmin) / (n - 1) as f32
    } else {
        0.0
    };
    for i in 0..n {
        let t = tmin + step * i as f32;
        coords.push(eval_cubic(coeff, t));
        tangents.push(eval_cubic_tangent(coeff, t).normalize_or_zero());
    }
    (coords, tangents)
}

fn eval_cubic(coeff: &[Vec3; 4], t: f32) -> Vec3 {
    coeff[0] + coeff[1] * t + coeff[2] * t * t + coeff[3] * t * t * t
}

fn eval_cubic_tangent(coeff: &[Vec3; 4], t: f32) -> Vec3 {
    coeff[1] + coeff[2] * (2.0 * t) + coeff[3] * (3.0 * t * t)
}

fn path_plane_normals(coords: &[Vec3], tangents: &[Vec3]) -> Vec<Vec3> {
    let n = coords.len();
    let mut normals = vec![Vec3::ZERO; n];
    if n < 2 {
        return normals;
    }

    for i in 1..(n - 1) {
        normals[i] = (coords[i - 1] - coords[i]).cross(coords[i + 1] - coords[i]);
    }
    normals[0] = normals[1];
    normals[n - 1] = normals[n - 2];

    let mut prev = None;
    for i in 0..n {
        let mut normal = project_perpendicular(normals[i], tangents[i]);
        let len = normal.length();
        if len > 1e-6 {
            if prev.is_some_and(|p: Vec3| normal.dot(p) < 0.0) {
                normal = -normal;
            }
            normal /= len;
            normals[i] = normal;
            prev = Some(normal);
        } else {
            normals[i] = Vec3::ZERO;
        }
    }
    replace_zero_normals(&mut normals, tangents);
    normals
}

fn replace_zero_normals(normals: &mut [Vec3], tangents: &[Vec3]) {
    let n = normals.len();
    let mut i = 0;
    while i < n {
        if normals[i].length_squared() > 1e-12 {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && normals[i].length_squared() <= 1e-12 {
            i += 1;
        }
        let end = i;
        if start == 0 && end < n {
            let fill = normals[end];
            for normal in normals.iter_mut().take(end) {
                *normal = fill;
            }
        } else if start > 0 && end < n {
            let n0 = normals[start - 1];
            let n1 = normals[end];
            for (offset, normal) in normals[start..end].iter_mut().enumerate() {
                let f = (offset + 1) as f32 / (end - start + 1) as f32;
                *normal = n0 * (1.0 - f) + n1 * f;
            }
        } else if start > 0 {
            let fill = normals[start - 1];
            for normal in normals.iter_mut().take(end).skip(start) {
                *normal = fill;
            }
        } else {
            for j in 0..n {
                normals[j] = fallback_normal(tangents[j]);
            }
        }
    }

    for i in 0..n {
        let normal = project_perpendicular(normals[i], tangents[i]).normalize_or_zero();
        normals[i] = if normal.length_squared() > 1e-12 {
            normal
        } else {
            fallback_normal(tangents[i])
        };
    }
}

fn parallel_transport_normals(tangents: &[Vec3], start_normal: Vec3, backwards: bool) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; tangents.len()];
    if tangents.is_empty() {
        return normals;
    }

    if backwards {
        let reversed_tangents: Vec<Vec3> = tangents.iter().rev().copied().collect();
        let mut reversed = parallel_transport_normals(&reversed_tangents, start_normal, false);
        reversed.reverse();
        return reversed;
    }

    let mut normal = project_perpendicular(start_normal, tangents[0]).normalize_or_zero();
    if normal.length_squared() <= 1e-12 {
        normal = fallback_normal(tangents[0]);
    }
    normals[0] = normal;

    for i in 1..tangents.len() {
        let prev_t = tangents[i - 1].normalize_or_zero();
        let curr_t = tangents[i].normalize_or_zero();
        let axis = prev_t.cross(curr_t);
        if axis.length_squared() > 1e-12 {
            let angle = prev_t.angle_between(curr_t);
            normal = Quat::from_axis_angle(axis.normalize(), angle) * normal;
        }
        normal = project_perpendicular(normal, curr_t).normalize_or_zero();
        if normal.length_squared() <= 1e-12 {
            normal = fallback_normal(curr_t);
        }
        normals[i] = normal;
    }

    normals
}

fn smooth_twist(tangents: &[Vec3], normals: &mut [Vec3], end_normal: Vec3) {
    if tangents.len() < 2 || normals.is_empty() {
        return;
    }
    let last = tangents.len() - 1;
    let angle = dihedral_angle(normals[last], end_normal, tangents[last]);
    for i in 0..normals.len() {
        let f = i as f32 / last as f32;
        let q = Quat::from_axis_angle(tangents[i].normalize_or_zero(), angle * f);
        normals[i] = (q * normals[i]).normalize_or_zero();
    }
}

fn need_normal_flip(transported_normal: Vec3, end_normal: Vec3, tangent: Vec3) -> bool {
    dihedral_angle(transported_normal, end_normal, tangent).abs() > 0.6 * std::f32::consts::PI
}

fn dihedral_angle(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let axis = axis.normalize_or_zero();
    let from = project_perpendicular(from, axis).normalize_or_zero();
    let to = project_perpendicular(to, axis).normalize_or_zero();
    axis.dot(from.cross(to)).atan2(from.dot(to))
}

fn ribbon_extrusions(
    coords: &[Vec3],
    tangents: &[Vec3],
    normals: &[Vec3],
    ranges: &[RibbonRange],
    num_res: usize,
    xs_front: &[RibbonXSection],
    xs_back: &[RibbonXSection],
    residue_colors: Option<&[Vec4]>,
    manager: &ChimeraXSectionManager,
) -> MeshData {
    let mut mesh = MeshData::default();
    if num_res == 0 || coords.is_empty() {
        return mesh;
    }

    let nsp = coords.len() / num_res;
    let nlp = nsp / 2;
    let nrp = (nsp + 1) / 2;

    for range in ranges {
        let mut capped = true;
        for i in range.start..range.end {
            let residue_color = residue_colors.and_then(|colors| colors.get(i)).copied();
            let mid_cap = xs_front[i].kind != xs_back[i].kind;
            let s = i * nsp;
            let e = s + nlp + 1;
            let mut front_mesh = xs_front[i].extrude(
                &coords[s..e],
                &tangents[s..e],
                &normals[s..e],
                capped,
                mid_cap,
                manager,
            );
            if let Some(color) = residue_color {
                front_mesh.colors = Some(vec![color; front_mesh.vertices.len()]);
            }
            mesh.append(&front_mesh);

            let next_cap = if i + 1 == range.end {
                true
            } else {
                xs_back[i].kind != xs_front[i + 1].kind
            };
            let s = i * nsp + nlp;
            let e = if i < num_res - 1 {
                s + nrp + 1
            } else {
                s + nrp
            };
            let mut back_mesh = xs_back[i].extrude(
                &coords[s..e],
                &tangents[s..e],
                &normals[s..e],
                mid_cap,
                next_cap,
                manager,
            );
            if let Some(color) = residue_color {
                back_mesh.colors = Some(vec![color; back_mesh.vertices.len()]);
            }
            mesh.append(&back_mesh);
            capped = next_cap;
        }
    }

    mesh
}

impl RibbonXSection {
    fn extrude(
        self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals_3d: &[Vec3],
        cap_front: bool,
        cap_back: bool,
        manager: &ChimeraXSectionManager,
    ) -> MeshData {
        let section = section_coords(self.kind, manager);
        if section.faceted {
            extrude_faceted(&section, centers, tangents, normals_3d, cap_front, cap_back)
        } else {
            extrude_smooth(&section, centers, tangents, normals_3d, cap_front, cap_back)
        }
    }
}

struct SectionGeometry {
    coords: Vec<[f32; 2]>,
    coords2: Option<Vec<[f32; 2]>>,
    normals: Vec<[f32; 2]>,
    normals2: Option<Vec<[f32; 2]>>,
    faceted: bool,
    tessellation: Vec<[u32; 3]>,
}

fn section_coords(kind: RibbonRef, _manager: &ChimeraXSectionManager) -> SectionGeometry {
    match kind {
        RibbonRef::Coil => xsection_round((0.2, 0.2), 12, false),
        RibbonRef::Helix => xsection_round((1.0, 0.2), 12, false),
        RibbonRef::HelixArrow => xsection_round((1.0, 0.2), 12, false),
        RibbonRef::Sheet => xsection_square((1.0, 0.2)),
        RibbonRef::SheetArrow => xsection_square_arrow((2.0, 0.2), (0.2, 0.2)),
    }
}

fn xsection_round(scale: (f32, f32), sides: usize, faceted: bool) -> SectionGeometry {
    let mut coords = Vec::with_capacity(sides);
    let mut normals = Vec::with_capacity(sides);
    for i in 0..sides {
        let theta = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let x = theta.cos();
        let y = theta.sin();
        coords.push([x * scale.0, y * scale.1]);
        normals.push([x * scale.1, y * scale.0]);
    }
    normalize_section_normals(&mut normals);
    SectionGeometry {
        coords,
        coords2: None,
        normals,
        normals2: None,
        faceted,
        tessellation: fan_tessellation(sides),
    }
}

fn xsection_square(scale: (f32, f32)) -> SectionGeometry {
    let coords = vec![
        [scale.0, scale.1],
        [-scale.0, scale.1],
        [-scale.0, -scale.1],
        [scale.0, -scale.1],
    ];
    let (normals, normals2) = generate_faceted_section_normals(&coords);
    SectionGeometry {
        coords,
        coords2: None,
        normals,
        normals2: Some(normals2),
        faceted: true,
        tessellation: vec![[0, 1, 2], [0, 2, 3]],
    }
}

fn xsection_square_arrow(start: (f32, f32), end: (f32, f32)) -> SectionGeometry {
    let mut section = xsection_square(start);
    section.coords2 = Some(vec![
        [end.0, end.1],
        [-end.0, end.1],
        [-end.0, -end.1],
        [end.0, -end.1],
    ]);
    section
}

fn generate_faceted_section_normals(coords: &[[f32; 2]]) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
    let num_coords = coords.len();
    let mut normals = vec![[0.0, 0.0]; num_coords];
    let mut normals2 = vec![[0.0, 0.0]; num_coords];
    for i in 0..num_coords {
        let j = (i + 1) % num_coords;
        let dx = coords[j][0] - coords[i][0];
        let dy = coords[j][1] - coords[i][1];
        normals[i] = [dy, -dx];
        normals2[j] = [dy, -dx];
    }
    normalize_section_normals(&mut normals);
    normalize_section_normals(&mut normals2);
    (normals, normals2)
}

fn normalize_section_normals(normals: &mut [[f32; 2]]) {
    for normal in normals {
        let len = (normal[0] * normal[0] + normal[1] * normal[1])
            .sqrt()
            .max(1e-6);
        normal[0] /= len;
        normal[1] /= len;
    }
}

fn fan_tessellation(n: usize) -> Vec<[u32; 3]> {
    (1..(n - 1))
        .map(|i| [0, i as u32, (i + 1) as u32])
        .collect()
}

fn extrude_smooth(
    section: &SectionGeometry,
    centers: &[Vec3],
    tangents: &[Vec3],
    normals_3d: &[Vec3],
    cap_front: bool,
    cap_back: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    let ns = section.coords.len();
    for j in 0..ns {
        let cp1 = section.coords[j];
        let cp2 = section.coords2.as_ref().map(|coords| coords[j]);
        let np = section.normals[j];
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), false);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            mesh.vertices
                .push(centers[i] + normals_3d[i] * cn + binormal * cb);
            mesh.normals
                .push((normals_3d[i] * np[0] + binormal * np[1]).normalize_or_zero());
        }
    }
    add_side_triangles(&mut mesh, ns, centers.len(), false);
    add_section_caps(
        &mut mesh,
        section,
        tangents,
        ns,
        centers.len(),
        cap_front,
        cap_back,
    );
    mesh
}

fn extrude_faceted(
    section: &SectionGeometry,
    centers: &[Vec3],
    tangents: &[Vec3],
    normals_3d: &[Vec3],
    cap_front: bool,
    cap_back: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    let ns = section.coords.len();
    let normals2 = section.normals2.as_ref().unwrap_or(&section.normals);
    for j in 0..ns {
        let cp1 = section.coords[j];
        let cp2 = section.coords2.as_ref().map(|coords| coords[j]);
        let np1 = section.normals[j];
        let np2 = normals2[j];
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), true);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            let pos = centers[i] + normals_3d[i] * cn + binormal * cb;
            mesh.vertices.push(pos);
            mesh.normals
                .push((normals_3d[i] * np1[0] + binormal * np1[1]).normalize_or_zero());
        }
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), true);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            let pos = centers[i] + normals_3d[i] * cn + binormal * cb;
            mesh.vertices.push(pos);
            mesh.normals
                .push((normals_3d[i] * np2[0] + binormal * np2[1]).normalize_or_zero());
        }
    }
    add_side_triangles(&mut mesh, ns, centers.len(), true);
    add_section_caps(
        &mut mesh,
        section,
        tangents,
        ns,
        centers.len(),
        cap_front,
        cap_back,
    );
    mesh
}

fn interpolated_coord(
    cp1: [f32; 2],
    cp2: Option<[f32; 2]>,
    i: usize,
    n: usize,
    faceted: bool,
) -> (f32, f32) {
    let Some(cp2) = cp2 else {
        return (cp1[0], cp1[1]);
    };
    let arrow_len = if faceted { n.saturating_sub(1) } else { n };
    let t = if arrow_len == 0 {
        1.0
    } else {
        (i.min(arrow_len) as f32) / arrow_len as f32
    };
    (
        cp1[0] * (1.0 - t) + cp2[0] * t,
        cp1[1] * (1.0 - t) + cp2[1] * t,
    )
}

fn add_side_triangles(mesh: &mut MeshData, ns: usize, np: usize, faceted: bool) {
    for s in 0..ns {
        let i_start = if faceted { s * 2 * np } else { s * np };
        let j = (s + 1) % ns;
        let j_start = if faceted { (j * 2 + 1) * np } else { j * np };
        for k in 0..(np - 1) {
            mesh.indices.extend_from_slice(&[
                (i_start + k + 1) as u32,
                (i_start + k) as u32,
                (j_start + k) as u32,
                (i_start + k + 1) as u32,
                (j_start + k) as u32,
                (j_start + k + 1) as u32,
            ]);
        }
    }
}

fn add_section_caps(
    mesh: &mut MeshData,
    section: &SectionGeometry,
    tangents: &[Vec3],
    ns: usize,
    np: usize,
    cap_front: bool,
    cap_back: bool,
) {
    if cap_front {
        let source_indices: Vec<usize> = if section.faceted {
            (0..ns).map(|i| i * 2 * np).collect()
        } else {
            (0..ns).map(|i| i * np).collect()
        };
        add_cap_from_existing(mesh, section, &source_indices, -tangents[0], true);
    }
    if cap_back {
        let last = np - 1;
        let source_indices: Vec<usize> = if section.faceted {
            (0..ns).map(|i| i * 2 * np + last).collect()
        } else {
            (0..ns).map(|i| i * np + last).collect()
        };
        add_cap_from_existing(mesh, section, &source_indices, tangents[last], false);
    }
}

fn add_cap_from_existing(
    mesh: &mut MeshData,
    section: &SectionGeometry,
    source_indices: &[usize],
    tangent: Vec3,
    reverse: bool,
) {
    let base = mesh.vertices.len() as u32;
    let tangent = tangent.normalize_or_zero();
    let cap_vertices: Vec<Vec3> = source_indices
        .iter()
        .map(|&index| mesh.vertices[index])
        .collect();
    for vertex in cap_vertices {
        mesh.vertices.push(vertex);
        mesh.normals.push(tangent);
    }
    for tri in &section.tessellation {
        if reverse {
            mesh.indices
                .extend_from_slice(&[base + tri[2], base + tri[1], base + tri[0]]);
        } else {
            mesh.indices
                .extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
        }
    }
}

fn fallback_normal(tangent: Vec3) -> Vec3 {
    if tangent.dot(Vec3::Z).abs() < 0.98 {
        tangent.cross(Vec3::Z).normalize_or_zero()
    } else {
        tangent.cross(Vec3::X).normalize_or_zero()
    }
}

fn project_perpendicular(vector: Vec3, tangent: Vec3) -> Vec3 {
    vector - tangent * vector.dot(tangent)
}
