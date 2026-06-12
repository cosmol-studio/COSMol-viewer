// Derived from UCSF ChimeraX atomstruct_cpp/Atom.cpp and
// element_cpp/Element.cpp.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

pub(crate) fn default_radius(
    atomic_number: u8,
    idatm_type: Option<&str>,
    bond_count: usize,
    has_hydrogen_neighbor: bool,
    structure_has_hydrogens: bool,
) -> f32 {
    if structure_has_hydrogens
        && idatm_info(idatm_type).is_some_and(|(_, substituents)| substituents <= bond_count)
    {
        match atomic_number {
            1 => return 1.00,
            6 => return 1.70,
            7 => return 1.625,
            8 => {
                let geometry = idatm_info(idatm_type).map_or(0, |(geometry, _)| geometry);
                return if geometry < 4 { 1.48 } else { 1.50 };
            }
            9 => return 1.56,
            15 => return 1.871,
            16 => return 1.782,
            17 => return 1.735,
            35 => return 1.978,
            53 => return 2.094,
            _ => {}
        }
    }

    let coordination = if bond_count == 0 { 6 } else { bond_count };
    match atomic_number {
        1 => 1.0,
        3 => {
            if coordination <= 4 {
                0.59
            } else if coordination >= 8 {
                0.92
            } else {
                0.76
            }
        }
        4 => {
            if coordination <= 4 {
                0.27
            } else {
                0.45
            }
        }
        6 => {
            if !matches!(idatm_type, Some("Car" | "C2")) {
                1.88
            } else if bond_count < 3 || (structure_has_hydrogens && has_hydrogen_neighbor) {
                1.76
            } else {
                1.61
            }
        }
        7 => 1.64,
        8 => {
            if idatm_type == Some("O3") {
                1.46
            } else {
                1.42
            }
        }
        9 if bond_count == 0 => 1.33,
        11 => {
            if coordination <= 4 {
                0.99
            } else if coordination >= 12 {
                1.39
            } else if coordination >= 9 {
                1.24
            } else if coordination >= 8 {
                1.18
            } else {
                1.02
            }
        }
        12 => {
            if coordination <= 4 {
                0.57
            } else if coordination >= 8 {
                0.89
            } else {
                0.72
            }
        }
        13 => {
            if coordination <= 4 {
                0.39
            } else if coordination >= 6 {
                0.54
            } else {
                0.48
            }
        }
        15 => 2.1,
        16 => 1.77,
        17 if bond_count == 0 => 1.81,
        19 => {
            if coordination <= 4 {
                1.37
            } else if coordination >= 12 {
                1.64
            } else if coordination >= 10 {
                1.51
            } else {
                1.38
            }
        }
        20 => {
            if coordination >= 12 {
                1.34
            } else if coordination >= 10 {
                1.23
            } else if coordination >= 8 {
                1.12
            } else {
                1.00
            }
        }
        21 => {
            if coordination >= 8 {
                0.87
            } else {
                0.75
            }
        }
        22 => 0.86,
        23 => 0.79,
        24 => 0.73,
        25 => {
            if coordination <= 4 {
                0.66
            } else if coordination >= 8 {
                0.96
            } else {
                0.83
            }
        }
        26 => {
            if coordination <= 4 {
                0.63
            } else if coordination >= 8 {
                0.92
            } else {
                0.61
            }
        }
        27 => {
            if coordination <= 4 {
                0.56
            } else if coordination >= 8 {
                0.90
            } else {
                0.65
            }
        }
        28 => {
            if coordination <= 4 {
                0.49
            } else {
                0.69
            }
        }
        29 => {
            if coordination <= 4 {
                0.57
            } else {
                0.73
            }
        }
        30 => {
            if coordination <= 4 {
                0.60
            } else if coordination >= 8 {
                0.90
            } else {
                0.74
            }
        }
        31 => {
            if coordination <= 4 {
                0.47
            } else {
                0.62
            }
        }
        32 => 0.73,
        33 => 0.58,
        34 => 0.50,
        35 if bond_count == 0 => 1.96,
        37 => {
            if coordination >= 12 {
                1.72
            } else if coordination >= 10 {
                1.66
            } else if coordination >= 8 {
                1.61
            } else {
                1.52
            }
        }
        38 => {
            if coordination >= 12 {
                1.44
            } else if coordination >= 10 {
                1.36
            } else if coordination >= 8 {
                1.26
            } else {
                1.18
            }
        }
        39 => {
            if coordination >= 9 {
                1.08
            } else if coordination >= 8 {
                1.02
            } else {
                0.90
            }
        }
        40 => {
            if coordination <= 4 {
                0.59
            } else if coordination >= 9 {
                0.89
            } else if coordination >= 8 {
                0.84
            } else {
                0.72
            }
        }
        41 => {
            if coordination >= 8 {
                0.79
            } else {
                0.72
            }
        }
        42 => 0.69,
        43 => 0.65,
        44 => 0.68,
        45 => 0.67,
        46 => {
            if coordination <= 4 {
                0.64
            } else {
                0.86
            }
        }
        47 => {
            if coordination <= 4 {
                0.79
            } else {
                0.94
            }
        }
        48 => {
            if coordination <= 4 {
                0.78
            } else if coordination >= 12 {
                1.31
            } else if coordination >= 8 {
                1.10
            } else {
                0.95
            }
        }
        49 => {
            if coordination <= 4 {
                0.62
            } else {
                0.80
            }
        }
        50 => {
            if coordination <= 4 {
                0.55
            } else if coordination >= 8 {
                0.81
            } else {
                0.69
            }
        }
        51 => 0.76,
        52 => {
            if coordination <= 4 {
                0.66
            } else {
                0.97
            }
        }
        53 if bond_count == 0 => 2.20,
        55 => {
            if coordination >= 12 {
                1.88
            } else if coordination >= 10 {
                1.81
            } else if coordination >= 8 {
                1.74
            } else {
                1.67
            }
        }
        56 => {
            if coordination >= 12 {
                1.61
            } else if coordination >= 8 {
                1.42
            } else {
                1.35
            }
        }
        57 => {
            if coordination >= 12 {
                1.36
            } else if coordination >= 10 {
                1.27
            } else if coordination >= 8 {
                1.16
            } else {
                1.03
            }
        }
        58 => {
            if coordination >= 12 {
                1.34
            } else if coordination >= 10 {
                1.25
            } else if coordination >= 8 {
                1.14
            } else {
                1.01
            }
        }
        59 => {
            if coordination >= 8 {
                1.13
            } else {
                0.99
            }
        }
        60 => {
            if coordination >= 12 {
                1.27
            } else if coordination >= 9 {
                1.16
            } else if coordination >= 8 {
                1.12
            } else {
                0.98
            }
        }
        61 => {
            if coordination >= 8 {
                1.09
            } else {
                0.97
            }
        }
        62 => {
            if coordination >= 8 {
                1.27
            } else {
                1.19
            }
        }
        63 => {
            if coordination >= 10 {
                1.35
            } else if coordination >= 8 {
                1.25
            } else {
                1.17
            }
        }
        64 => {
            if coordination >= 8 {
                1.05
            } else {
                0.94
            }
        }
        65 => {
            if coordination >= 8 {
                1.04
            } else {
                0.92
            }
        }
        66 => {
            if coordination >= 8 {
                1.19
            } else {
                1.07
            }
        }
        67 => 0.901,
        68 => {
            if coordination >= 8 {
                1.00
            } else {
                0.89
            }
        }
        69 => {
            if coordination >= 7 {
                1.09
            } else {
                1.01
            }
        }
        70 => {
            if coordination >= 8 {
                1.14
            } else {
                1.02
            }
        }
        71 => {
            if coordination >= 8 {
                0.97
            } else {
                0.86
            }
        }
        72 => {
            if coordination <= 4 {
                0.58
            } else if coordination >= 8 {
                0.83
            } else {
                0.71
            }
        }
        73 => 0.72,
        74 => 0.66,
        75 => 0.63,
        76 => 0.63,
        77 => 0.68,
        78 => {
            if coordination <= 4 {
                0.60
            } else {
                0.80
            }
        }
        79 => 1.37,
        80 => {
            if coordination <= 2 {
                0.69
            } else if coordination <= 4 {
                0.96
            } else if coordination >= 8 {
                1.14
            } else {
                1.02
            }
        }
        81 => {
            if coordination >= 12 {
                1.70
            } else if coordination >= 8 {
                1.59
            } else {
                1.50
            }
        }
        82 => {
            if coordination >= 12 {
                1.49
            } else if coordination >= 10 {
                1.40
            } else if coordination >= 8 {
                1.29
            } else {
                1.19
            }
        }
        83 => {
            if coordination <= 5 {
                0.96
            } else if coordination >= 8 {
                1.17
            } else {
                1.03
            }
        }
        84 => 0.97,
        87 => 1.80,
        88 => {
            if coordination >= 10 {
                1.70
            } else {
                1.48
            }
        }
        89 => 1.12,
        90 => {
            if coordination >= 12 {
                1.21
            } else if coordination >= 10 {
                1.13
            } else if coordination >= 8 {
                1.05
            } else {
                0.94
            }
        }
        91 => 1.04,
        92 => 1.03,
        93 => 1.01,
        94 => 1.00,
        95 => {
            if coordination >= 8 {
                1.09
            } else {
                0.98
            }
        }
        96 => 0.97,
        97 => 0.96,
        98 => 0.95,
        99 => 0.835,
        _ => {
            let radius = 2.0 * bond_radius(atomic_number);
            if radius == 0.0 { 1.8 } else { radius }
        }
    }
}

fn idatm_info(idatm_type: Option<&str>) -> Option<(u8, usize)> {
    Some(match idatm_type? {
        "Car" => (3, 3),
        "C3" => (4, 4),
        "C2" => (3, 3),
        "C1" => (2, 2),
        "C1-" => (2, 1),
        "Cac" => (3, 3),
        "N3+" => (4, 4),
        "N3" => (4, 3),
        "Npl" => (3, 3),
        "N2+" => (3, 3),
        "N2" => (3, 2),
        "N1+" => (2, 2),
        "N1" => (2, 1),
        "Ntr" => (3, 3),
        "Ng+" => (3, 3),
        "O3" => (4, 2),
        "O3-" => (4, 1),
        "Oar" | "Oar+" => (3, 2),
        "O2" | "O2-" | "O1" | "O1+" => (3, 1),
        "S3+" => (4, 3),
        "S3" | "Sar" => (4, 2),
        "S3-" | "S2" => (4, 1),
        "Sac" | "Son" | "Pac" | "Pox" | "P3+" => (4, 4),
        "Sxd" => (4, 3),
        "HC" | "H" | "DC" | "D" => (1, 1),
        _ => return None,
    })
}

fn bond_radius(atomic_number: u8) -> f32 {
    const RADII: &[f32] = &[
        0.00, 0.23, 0.00, 0.68, 0.35, 0.83, 0.68, 0.68, 0.68, 0.64, 0.00, 0.97, 1.10, 1.35, 1.20,
        1.05, 1.02, 0.99, 0.00, 1.33, 0.99, 1.44, 1.47, 1.33, 1.35, 1.35, 1.34, 1.33, 1.50, 1.52,
        1.45, 1.22, 1.17, 1.21, 1.22, 1.21, 0.00, 1.47, 1.12, 1.78, 1.56, 1.48, 1.47, 1.35, 1.40,
        1.45, 1.50, 1.59, 1.69, 1.63, 1.46, 1.46, 1.47, 1.40, 0.00, 1.67, 1.34, 1.87, 1.83, 1.82,
        1.81, 1.80, 1.80, 1.99, 1.79, 1.76, 1.75, 1.74, 1.73, 1.72, 1.94, 1.72, 1.57, 1.43, 1.37,
        1.35, 1.37, 1.32, 1.50, 1.50, 1.70, 1.55, 1.54, 1.54, 1.68, 0.00, 0.00, 0.00, 1.90, 1.88,
        1.79, 1.61, 1.58, 1.55, 1.53, 1.51,
    ];
    RADII.get(atomic_number as usize).copied().unwrap_or(0.0)
}
