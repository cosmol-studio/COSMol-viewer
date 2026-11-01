//! COSMolKit transport only: chemistry stays in COSMolKit, rendering in core.
use crate::external::{External, ExternalType};
use cosmol_viewer_core::shapes::Molecule;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub(crate) struct MoleculeType;
impl ExternalType for MoleculeType {
    const MODULE: &'static str = "cosmolkit";
    const NAME: &'static str = "Molecule";
}
pub(crate) type MoleculeInput<'py> = External<'py, MoleculeType>;

// COSMolKit's current properties and its previous zero-argument accessors carry
// the same values. Select by representation, never by catching a failed read.
fn read<'py>(object: &Bound<'py, PyAny>, member: &str) -> PyResult<Bound<'py, PyAny>> {
    let value = object.getattr(member)?;
    if value.is_callable() {
        value.call0()
    } else {
        Ok(value)
    }
}

pub(super) fn into_viewer_molecule(input: MoleculeInput<'_>) -> PyResult<Molecule> {
    let molecule = input.as_bound().call_method0("with_kekulized_bonds")?;
    let atom_numbers = read(&molecule, "atoms")?
        .try_iter()?
        .map(|atom| read(&atom?, "atomic_number")?.extract::<u8>())
        .collect::<PyResult<Vec<_>>>()?;

    let conformers = read(&molecule, "conformers_3d")?;
    let first = conformers.try_iter()?.next().transpose()?;
    let (coordinates, dimension) = if let Some(conformer) = first {
        (read(&conformer, "coordinates")?, 3)
    } else {
        let coordinates = read(&molecule, "coordinates_2d")?;
        if coordinates.is_none() {
            let laid_out = molecule.call_method0("with_2d_coordinates")?;
            (read(&laid_out, "coordinates_2d")?, 2)
        } else {
            (coordinates, 2)
        }
    };
    let positions = coordinate_rows(&coordinates, atom_numbers.len(), dimension)?;
    let mut orders = Vec::new();
    let mut indices = Vec::new();
    for bond in read(&molecule, "bonds")?.try_iter()? {
        let bond = bond?;
        orders.push(read(&bond, "order_code")?.extract()?);
        indices.push([
            read(&bond, "begin")?.extract()?,
            read(&bond, "end")?.extract()?,
        ]);
    }
    Molecule::from_atom_bond_data(atom_numbers, positions, orders, indices, None)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

fn coordinate_rows(
    coordinates: &Bound<'_, PyAny>,
    count: usize,
    dimension: usize,
) -> PyResult<Vec<[f32; 3]>> {
    let rows: Vec<Vec<f64>> = coordinates.call_method0("tolist")?.extract()?;
    if rows.len() != count {
        return Err(PyValueError::new_err(format!(
            "COSMolKit exposed {} coordinates for {count} atoms",
            rows.len(),
        )));
    }
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            if row.len() != dimension {
                return Err(PyValueError::new_err(format!(
                    "coordinate row {index} has {} columns, expected {dimension}",
                    row.len(),
                )));
            }
            let mut position = [0.; 3];
            for (output, input) in position.iter_mut().zip(row) {
                *output = input as f32;
                if !output.is_finite() {
                    return Err(PyValueError::new_err(format!(
                        "coordinate row {index} must contain finite f32 values",
                    )));
                }
            }
            Ok(position)
        })
        .collect()
}
