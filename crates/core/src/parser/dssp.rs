// DSSP-style secondary structure assignment.
//
// This is a Rust port of the core algorithm in ChimeraX CompSS.cpp:
// https://github.com/RBVI/ChimeraX/blob/develop/src/bundles/atomic_lib/atomic_cpp/atomstruct_cpp/CompSS.cpp
//
// Original algorithm: Kabsch & Sander, Biopolymers 22:2577-2637 (1983).
//
// Porting map from CompSS.cpp:
// - add_imide_hydrogen/add_imide_hydrogens -> prepare_coords + calculate_imide_hydrogen
// - hbonded_to/find_hbonds -> hbonded_to + find_hbonds
// - find_turns/mark_helices/find_helices -> same-named Rust methods
// - find_bridges/find_beta_bulge/merge_bulge -> find_bridges + find_beta_bulge + merge_bulge
// - compute_chain markup -> assign_ribbon_info
//
// Source-reproduction note:
// The alignment blocks below use local CompSS.cpp line numbers and per-line behavior markers.
// They do not embed the ChimeraX C++ body verbatim; each referenced line is paraphrased as a
// behavior claim and must stay adjacent to the Rust code that implements it.

use crate::parser::utils::{Residue, RibbonResidueInfo, SecondaryStructure};
use glam::Vec3;
use na_seq::AminoAcid;

type HBondMatrix = Vec<Vec<bool>>;

#[derive(Clone, Copy)]
struct ResidueCoords {
    c: Vec3,
    n: Vec3,
    ca: Vec3,
    o: Vec3,
    h: Option<Vec3>,
    is_proline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LadderType {
    Parallel,
    Antiparallel,
}

#[derive(Debug, Clone, Copy)]
struct BetaLadder {
    ladder_type: LadderType,
    start: [usize; 2],
    end: [usize; 2],
    is_bulge: bool,
}

impl BetaLadder {
    fn new(ladder_type: LadderType, s1: usize, e1: usize, s2: usize, e2: usize) -> Self {
        let (start0, end0) = ordered_pair(s1, e1);
        let (start1, end1) = ordered_pair(s2, e2);
        Self {
            ladder_type,
            start: [start0, start1],
            end: [end0, end1],
            is_bulge: false,
        }
    }

    fn strand_len(&self, strand: usize) -> usize {
        self.end[strand] - self.start[strand] + 1
    }
}

pub struct SecondaryStructureCalculator {
    pub hbond_cutoff: f32,
    pub min_helix_length: usize,
    pub min_strand_length: usize,
}

impl Default for SecondaryStructureCalculator {
    fn default() -> Self {
        Self {
            hbond_cutoff: -0.5,
            min_helix_length: 3,
            min_strand_length: 2,
        }
    }
}

impl SecondaryStructureCalculator {
    const DSSP_3DONOR: u32 = 0x0001;
    const DSSP_3ACCEPTOR: u32 = 0x0002;
    const DSSP_3GAP: u32 = 0x0004;
    const DSSP_3HELIX: u32 = 0x0008;

    const DSSP_4DONOR: u32 = 0x0010;
    const DSSP_4ACCEPTOR: u32 = 0x0020;
    const DSSP_4GAP: u32 = 0x0040;
    const DSSP_4HELIX: u32 = 0x0080;

    const DSSP_5DONOR: u32 = 0x0100;
    const DSSP_5ACCEPTOR: u32 = 0x0200;
    const DSSP_5GAP: u32 = 0x0400;
    const DSSP_5HELIX: u32 = 0x0800;

    const DSSP_PBRIDGE: u32 = 0x1000;
    const DSSP_ABRIDGE: u32 = 0x2000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute_secondary_structure(&self, residues: &[Residue]) -> Vec<SecondaryStructure> {
        self.compute_ribbon_info(residues)
            .into_iter()
            .map(|info| info.ss)
            .collect()
    }

    pub fn compute_ribbon_info(&self, residues: &[Residue]) -> Vec<RibbonResidueInfo> {
        if residues.len() < 2 {
            return vec![RibbonResidueInfo::default(); residues.len()];
        }

        // BEGIN CHIMERAX CPP BODY: compute_ribbon_info
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: compute_chain core call order
        // ChimeraX✔️✔️ CompSS.cpp: add synthesized imide hydrogens before bond search.
        // ChimeraX✔️✔️ CompSS.cpp: compute the acceptor-to-donor hydrogen-bond matrix next.
        // ChimeraX✔️✔️ CompSS.cpp: find 3-turns, then promote adjacent 3-turn acceptors to 3-10 helices.
        // ChimeraX✔️✔️ CompSS.cpp: find 4-turns, then promote adjacent 4-turn acceptors to alpha helices.
        // ChimeraX✔️✔️ CompSS.cpp: find 5-turns, then promote adjacent 5-turn acceptors to pi helices.
        // ChimeraX✔️✔️ CompSS.cpp: collapse helix marker runs into final helix ranges.
        // ChimeraX✔️❗ CompSS.cpp: find bridges and beta ladders; Rust uses an O(n^2) scan instead of AtomSearchTree.
        // ChimeraX❗❗ CompSS.cpp: apply chain markup into COSMolKit ribbon info, not ChimeraX residue attributes.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: compute_chain core call order
        // END CHIMERAX CPP BODY: compute_ribbon_info
        let coords = self.prepare_coords(residues);
        let hbonds = self.find_hbonds(&coords);
        let mut flags = vec![0u32; residues.len()];

        self.find_turns(3, &mut flags, &hbonds);
        self.mark_helices(3, &mut flags);
        self.find_turns(4, &mut flags, &hbonds);
        self.mark_helices(4, &mut flags);
        self.find_turns(5, &mut flags, &hbonds);
        self.mark_helices(5, &mut flags);

        let helices = self.find_helices(&flags);
        let ladders = self.find_bridges(&mut flags, &hbonds, &coords);
        self.assign_ribbon_info(residues.len(), &helices, &ladders, &flags)
    }

    fn prepare_coords(&self, residues: &[Residue]) -> Vec<ResidueCoords> {
        let mut coords: Vec<ResidueCoords> = residues
            .iter()
            .map(|residue| ResidueCoords {
                c: residue.c,
                n: residue.n,
                ca: residue.ca,
                o: residue.o,
                h: residue.h,
                is_proline: residue.residue_type == AminoAcid::Pro,
            })
            .collect();

        // BEGIN CHIMERAX CPP BODY: prepare_coords
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: add_imide_hydrogens
        // ChimeraX✔️✔️ CompSS.cpp:162 enters the helper with mutable coordinate state.
        // ChimeraX✔️✔️ CompSS.cpp:164 records the residue count.
        // ChimeraX✔️✔️ CompSS.cpp:165-166 returns immediately for an empty chain.
        // ChimeraX✔️✔️ CompSS.cpp:167-168 seeds previous residue and coordinate with index 0.
        // ChimeraX✔️✔️ CompSS.cpp:169 starts scanning at residue index 1.
        // ChimeraX✔️✔️ CompSS.cpp:170-171 loads current residue and coordinate by index.
        // ChimeraX❗✔️ CompSS.cpp:172 checks ChimeraX connectivity; Rust uses peptide C-N distance policy.
        // ChimeraX✔️✔️ CompSS.cpp:173 calls the imide-hydrogen placement helper.
        // ChimeraX✔️✔️ CompSS.cpp:174 skips storage when the helper reports no synthetic H.
        // ChimeraX❌❌ CompSS.cpp:175 stores allocated Coord for later deletion; Rust has no heap Coord ownership list.
        // ChimeraX✔️✔️ CompSS.cpp:176 attaches the synthesized H coordinate to the current residue coordinate cache.
        // ChimeraX✔️✔️ CompSS.cpp:179-180 advances previous residue and coordinate to current.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: add_imide_hydrogens
        // END CHIMERAX CPP BODY: prepare_coords
        for i in 1..coords.len() {
            if coords[i].h.is_some() || !peptide_connected(coords[i - 1].c, coords[i].n) {
                continue;
            }

            coords[i].h = calculate_imide_hydrogen(coords[i], coords[i - 1]);
        }

        coords
    }

    fn find_hbonds(&self, coords: &[ResidueCoords]) -> HBondMatrix {
        let num_res = coords.len();
        let mut hbonds = vec![vec![false; num_res]; num_res];

        // BEGIN CHIMERAX CPP BODY: find_hbonds
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_hbonds
        // ChimeraX✔️✔️ CompSS.cpp:220 records the residue count.
        // ChimeraX✔️✔️ CompSS.cpp:221 initializes a proline donor-suppression vector.
        // ChimeraX❗✔️ CompSS.cpp:223-232 marks proline by chemistry; Rust uses parsed amino-acid identity.
        // ChimeraX✔️✔️ CompSS.cpp:233 iterates each residue as an acceptor candidate.
        // ChimeraX✔️✔️ CompSS.cpp:234 loads acceptor coordinates for the current residue.
        // ChimeraX✔️❗ CompSS.cpp:235 searches N atoms within 10 A; Rust scans all later residues and filters by distance.
        // ChimeraX✔️❗ CompSS.cpp:236 maps the nearby atom back to residue index; Rust already has the index.
        // ChimeraX✔️✔️ CompSS.cpp:237-238 skips current, previous, and next residue candidates.
        // ChimeraX✔️✔️ CompSS.cpp:239 loads donor-side coordinates for the nearby residue.
        // ChimeraX✔️✔️ CompSS.cpp:240-244 suppresses proline donation, otherwise tests acceptor i against donor j.
        // ChimeraX✔️✔️ CompSS.cpp:245-248 suppresses reverse proline donation, otherwise tests acceptor j against donor i.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_hbonds
        // END CHIMERAX CPP BODY: find_hbonds
        for i in 0..num_res {
            for near_index in (i + 2)..num_res {
                if coords[i].n.distance_squared(coords[near_index].n) > 100.0 {
                    continue;
                }

                if !coords[near_index].is_proline {
                    hbonds[i][near_index] =
                        hbonded_to(coords[i], coords[near_index], self.hbond_cutoff);
                }

                if !coords[i].is_proline {
                    hbonds[near_index][i] =
                        hbonded_to(coords[near_index], coords[i], self.hbond_cutoff);
                }
            }
        }

        hbonds
    }

    fn find_turns(&self, n: usize, flags: &mut [u32], hbonds: &HBondMatrix) {
        let (donor, acceptor, gap) = match n {
            3 => (Self::DSSP_3DONOR, Self::DSSP_3ACCEPTOR, Self::DSSP_3GAP),
            4 => (Self::DSSP_4DONOR, Self::DSSP_4ACCEPTOR, Self::DSSP_4GAP),
            5 => (Self::DSSP_5DONOR, Self::DSSP_5ACCEPTOR, Self::DSSP_5GAP),
            _ => return,
        };

        // BEGIN CHIMERAX CPP BODY: find_turns
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_turns
        // ChimeraX✔️✔️ CompSS.cpp:257 receives the turn span n.
        // ChimeraX✔️✔️ CompSS.cpp:259-260 chooses the donor flag for n = 3, 4, or 5.
        // ChimeraX✔️✔️ CompSS.cpp:261-262 chooses the acceptor flag for n = 3, 4, or 5.
        // ChimeraX✔️✔️ CompSS.cpp:263-264 chooses the gap flag for n = 3, 4, or 5.
        // ChimeraX✔️✔️ CompSS.cpp:265 computes the last valid start index.
        // ChimeraX✔️✔️ CompSS.cpp:266 scans every possible i to i+n turn candidate.
        // ChimeraX✔️✔️ CompSS.cpp:267 requires an H-bond from acceptor i to donor i+n.
        // ChimeraX✔️✔️ CompSS.cpp:268 marks residue i as turn acceptor.
        // ChimeraX✔️✔️ CompSS.cpp:269-270 marks intervening residues as n-turn gap.
        // ChimeraX✔️✔️ CompSS.cpp:271 marks residue i+n as turn donor.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_turns
        // END CHIMERAX CPP BODY: find_turns
        let max = flags.len().saturating_sub(n);
        for i in 0..max {
            if hbonds[i][i + n] {
                flags[i] |= acceptor;
                for j in 1..n {
                    flags[i + j] |= gap;
                }
                flags[i + n] |= donor;
            }
        }
    }

    fn mark_helices(&self, n: usize, flags: &mut [u32]) {
        let (acceptor, helix) = match n {
            3 => (Self::DSSP_3ACCEPTOR, Self::DSSP_3HELIX),
            4 => (Self::DSSP_4ACCEPTOR, Self::DSSP_4HELIX),
            5 => (Self::DSSP_5ACCEPTOR, Self::DSSP_5HELIX),
            _ => return,
        };

        // BEGIN CHIMERAX CPP BODY: mark_helices
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: mark_helices
        // ChimeraX✔️✔️ CompSS.cpp:279 receives the turn span n.
        // ChimeraX✔️✔️ CompSS.cpp:281-282 chooses the acceptor flag for n = 3, 4, or 5.
        // ChimeraX✔️✔️ CompSS.cpp:283-284 chooses the helix flag for n = 3, 4, or 5.
        // ChimeraX✔️✔️ CompSS.cpp:285 computes the last valid helix seed index.
        // ChimeraX✔️✔️ CompSS.cpp:286 starts at index 1 so i-1 is valid.
        // ChimeraX✔️✔️ CompSS.cpp:287-288 requires consecutive acceptor markers at i-1 and i.
        // ChimeraX✔️✔️ CompSS.cpp:289-290 marks residues i through i+n-1 with the helix flag.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: mark_helices
        // END CHIMERAX CPP BODY: mark_helices
        let max = flags.len().saturating_sub(n);
        for i in 1..max {
            if (flags[i - 1] & acceptor) != 0 && (flags[i] & acceptor) != 0 {
                for j in 0..n {
                    flags[i + j] |= helix;
                }
            }
        }
    }

    fn find_helices(&self, flags: &[u32]) -> Vec<(usize, usize)> {
        // BEGIN CHIMERAX CPP BODY: find_helices
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_helices
        // ChimeraX✔️✔️ CompSS.cpp:302 stores residue count.
        // ChimeraX✔️✔️ CompSS.cpp:303 initializes the current helix start to none.
        // ChimeraX✔️✔️ CompSS.cpp:304 stores the current helix type once a run starts.
        // ChimeraX✔️✔️ CompSS.cpp:305 counts consecutive acceptor-only residues after the initial run.
        // ChimeraX✔️✔️ CompSS.cpp:306 tracks whether the run is still in its initial acceptor-only prefix.
        // ChimeraX✔️✔️ CompSS.cpp:307 scans all residues.
        // ChimeraX✔️✔️ CompSS.cpp:308 reads the residue flags.
        // ChimeraX✔️✔️ CompSS.cpp:309-311 resets per-residue helix type, relevant flags, and acceptor-only status.
        // ChimeraX✔️✔️ CompSS.cpp:312-315 maps 3-helix flags to 3-10 helix state.
        // ChimeraX✔️✔️ CompSS.cpp:316-320 maps 4- or 5-helix flags to alpha-family helix state.
        // ChimeraX✔️✔️ CompSS.cpp:322 requires both a helix type and one relevant turn flag.
        // ChimeraX✔️✔️ CompSS.cpp:323-326 starts a new helix run at the current residue.
        // ChimeraX✔️✔️ CompSS.cpp:327-332 closes the previous run when the helix type changes.
        // ChimeraX✔️✔️ CompSS.cpp:333-335 updates initial acceptor-only tracking for same-type continuation.
        // ChimeraX✔️✔️ CompSS.cpp:336-338 keeps an initial acceptor-only prefix from splitting the run.
        // ChimeraX✔️✔️ CompSS.cpp:338-348 lets one later acceptor-only residue pass, but splits at two.
        // ChimeraX✔️✔️ CompSS.cpp:349-351 clears acceptor-only count on a donor-containing residue.
        // ChimeraX✔️✔️ CompSS.cpp:352-357 closes a run when the current residue is no longer helix-marked.
        // ChimeraX✔️✔️ CompSS.cpp:359-362 closes a final run at end of chain.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_helices
        // END CHIMERAX CPP BODY: find_helices
        let mut helices = Vec::new();
        let mut first: Option<usize> = None;
        let mut cur_helix_type = 0;
        let mut acc_only_run = 0;
        let mut in_initial_acc_only = false;

        for i in 0..flags.len() {
            let f = flags[i];
            let mut helix_type = 0;
            let mut helix_flags = 0;
            let mut acc_only = false;

            if (f & Self::DSSP_3HELIX) != 0 {
                helix_type = 3;
                helix_flags = Self::DSSP_3ACCEPTOR | Self::DSSP_3DONOR | Self::DSSP_3GAP;
                acc_only = (f & Self::DSSP_3DONOR) == 0;
            } else if (f & (Self::DSSP_4HELIX | Self::DSSP_5HELIX)) != 0 {
                helix_type = 4;
                helix_flags = Self::DSSP_4ACCEPTOR
                    | Self::DSSP_4DONOR
                    | Self::DSSP_4GAP
                    | Self::DSSP_5ACCEPTOR
                    | Self::DSSP_5DONOR
                    | Self::DSSP_5GAP;
                acc_only = (f & Self::DSSP_4ACCEPTOR) != 0 && (f & Self::DSSP_4DONOR) == 0;
            }

            if helix_type != 0 && (f & helix_flags) != 0 {
                if first.is_none() {
                    first = Some(i);
                    cur_helix_type = helix_type;
                    in_initial_acc_only = true;
                } else if helix_type != cur_helix_type {
                    let start = first.unwrap();
                    if i - start >= self.min_helix_length {
                        helices.push((start, i - 1));
                    }
                    first = Some(i);
                    cur_helix_type = helix_type;
                    acc_only_run = 0;
                } else {
                    in_initial_acc_only = in_initial_acc_only && acc_only;
                }

                if in_initial_acc_only {
                    in_initial_acc_only = acc_only || Some(i) == first;
                } else if acc_only {
                    if acc_only_run > 0 {
                        let start = first.unwrap();
                        if i - 1 - start >= self.min_helix_length {
                            helices.push((start, i - 2));
                        }
                        first = Some(i - 1);
                        cur_helix_type = helix_type;
                        acc_only_run = 0;
                        in_initial_acc_only = true;
                    } else {
                        acc_only_run += 1;
                    }
                } else {
                    acc_only_run = 0;
                }
            } else if let Some(start) = first {
                if i - start >= self.min_helix_length {
                    helices.push((start, i - 1));
                }
                first = None;
                acc_only_run = 0;
            }
        }

        if let Some(start) = first {
            if flags.len() - start >= self.min_helix_length {
                helices.push((start, flags.len() - 1));
            }
        }

        helices
    }

    fn find_bridges(
        &self,
        flags: &mut [u32],
        hbonds: &HBondMatrix,
        coords: &[ResidueCoords],
    ) -> Vec<BetaLadder> {
        let max = flags.len();
        let mut bridge = vec![vec!['\0'; max]; max];

        // BEGIN CHIMERAX CPP BODY: find_bridges
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_bridges
        // ChimeraX✔️✔️ CompSS.cpp:454 stores residue count as a signed loop domain.
        // ChimeraX✔️✔️ CompSS.cpp:456-461 allocates a square bridge marker matrix.
        // ChimeraX✔️✔️ CompSS.cpp:463 declares the outer scan index.
        // ChimeraX✔️✔️ CompSS.cpp:464 scans i while i+1 remains valid.
        // ChimeraX✔️❗ CompSS.cpp:465-466 searches N atoms within 20 A; Rust scans all later residues and filters by distance.
        // ChimeraX✔️❗ CompSS.cpp:467 maps the nearby atom to residue index; Rust already has the index.
        // ChimeraX✔️✔️ CompSS.cpp:468-469 skips self and previously visited residue pairs.
        // ChimeraX✔️✔️ CompSS.cpp:470-472 detects a parallel bridge from the two offset H-bond patterns.
        // ChimeraX✔️✔️ CompSS.cpp:473-474 marks both participating residues with the parallel-bridge flag.
        // ChimeraX✔️✔️ CompSS.cpp:476-479 detects an antiparallel bridge from reciprocal or crossed offset H-bonds.
        // ChimeraX✔️✔️ CompSS.cpp:480-481 marks both participating residues with the antiparallel-bridge flag.
        // ChimeraX✔️✔️ CompSS.cpp:486-489 scans the bridge matrix to construct ladder runs.
        // ChimeraX✔️✔️ CompSS.cpp:490 switches on the unconsumed bridge marker.
        // ChimeraX✔️✔️ CompSS.cpp:491-495 consumes a parallel +/+ diagonal and appends one parallel ladder.
        // ChimeraX✔️✔️ CompSS.cpp:497-501 consumes an antiparallel +/- diagonal and appends one antiparallel ladder.
        // ChimeraX✔️✔️ CompSS.cpp:507-509 repeatedly merges beta-bulge-linked ladders until no merge remains.
        // ChimeraX✔️✔️ CompSS.cpp:511-520 removes ladders shorter than min_strand_length on either strand.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_bridges
        // END CHIMERAX CPP BODY: find_bridges
        for i in 0..max.saturating_sub(1) {
            for near_index in (i + 1)..max {
                if coords[i].n.distance_squared(coords[near_index].n) > 400.0 {
                    continue;
                }

                if (i > 0 && hbonds[i - 1][near_index] && hbonds[near_index][i + 1])
                    || (near_index < max - 1
                        && hbonds[near_index - 1][i]
                        && hbonds[i][near_index + 1])
                {
                    bridge[i][near_index] = 'P';
                    flags[i] |= Self::DSSP_PBRIDGE;
                    flags[near_index] |= Self::DSSP_PBRIDGE;
                } else if (hbonds[i][near_index] && hbonds[near_index][i])
                    || (i > 0
                        && near_index < max - 1
                        && hbonds[i - 1][near_index + 1]
                        && hbonds[near_index - 1][i + 1])
                {
                    bridge[i][near_index] = 'A';
                    flags[i] |= Self::DSSP_ABRIDGE;
                    flags[near_index] |= Self::DSSP_ABRIDGE;
                }
            }
        }

        let mut ladders = Vec::new();
        for i in 0..max {
            for j in (i + 1)..max {
                match bridge[i][j] {
                    'P' => {
                        let mut k = 0;
                        while i + k < max && j + k < max && bridge[i + k][j + k] == 'P' {
                            bridge[i + k][j + k] = 'p';
                            k += 1;
                        }
                        let k = k - 1;
                        ladders.push(BetaLadder::new(LadderType::Parallel, i, i + k, j, j + k));
                    }
                    'A' => {
                        let mut k = 0;
                        while i + k < max && j >= k && bridge[i + k][j - k] == 'A' {
                            bridge[i + k][j - k] = 'a';
                            k += 1;
                        }
                        let k = k - 1;
                        ladders.push(BetaLadder::new(
                            LadderType::Antiparallel,
                            i,
                            i + k,
                            j - k,
                            j,
                        ));
                    }
                    _ => {}
                }
            }
        }

        while find_beta_bulge(&mut ladders) {}

        ladders
            .into_iter()
            .filter(|ladder| {
                ladder.strand_len(0) >= self.min_strand_length
                    && ladder.strand_len(1) >= self.min_strand_length
            })
            .collect()
    }

    fn assign_ribbon_info(
        &self,
        num_residues: usize,
        helices: &[(usize, usize)],
        ladders: &[BetaLadder],
        flags: &[u32],
    ) -> Vec<RibbonResidueInfo> {
        let mut info = vec![RibbonResidueInfo::default(); num_residues];

        // BEGIN CHIMERAX CPP BODY: assign_ribbon_info
        // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: make_summary sheet grouping and residue summary
        // ChimeraX✔️✔️ CompSS.cpp:551-552 creates the sheet collection from merged ladders.
        // ChimeraX✔️✔️ CompSS.cpp:553 iterates every ladder.
        // ChimeraX✔️✔️ CompSS.cpp:554-567 expands both ladder strands into one residue-index set.
        // ChimeraX✔️✔️ CompSS.cpp:568-577 finds existing sheets that share at least one residue index.
        // ChimeraX✔️✔️ CompSS.cpp:578-584 merges all overlapping sheets and appends the new combined sheet.
        // ChimeraX❌❌ CompSS.cpp:587-599 assigns printable sheet letters; COSMolKit stores numeric sheet ids.
        // ChimeraX✔️✔️ CompSS.cpp:603-614 maps helix flags before bridge flags in the residue summary.
        // ChimeraX✔️✔️ CompSS.cpp:615-654 maps 3/4/5 turn flags; Rust exposes these as Turn only when no helix/sheet wins.
        // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: make_summary sheet grouping and residue summary
        // END CHIMERAX CPP BODY: assign_ribbon_info
        let sheet_groups = sheet_groups_from_ladders(ladders);
        for (sheet_id, sheet) in sheet_groups.iter().enumerate() {
            let mut ranges: Vec<(usize, usize)> = sheet
                .iter()
                .flat_map(|ladder| {
                    [
                        (ladder.start[0], ladder.end[0]),
                        (ladder.start[1], ladder.end[1]),
                    ]
                })
                .collect();
            ranges.sort_unstable();

            let mut merged_ranges: Vec<(usize, usize)> = Vec::new();
            for (start, end) in ranges {
                if let Some(last) = merged_ranges.last_mut() {
                    if start <= last.1 {
                        last.1 = last.1.max(end);
                        continue;
                    }
                }
                merged_ranges.push((start, end));
            }

            for (start, end) in merged_ranges {
                for residue_idx in start..=end {
                    info[residue_idx].ss = SecondaryStructure::Sheet;
                    info[residue_idx].sheet_id = Some(sheet_id);
                }
            }
        }

        for (helix_id, &(start, end)) in helices.iter().enumerate() {
            for residue_idx in start..=end {
                info[residue_idx].ss = SecondaryStructure::Helix;
                info[residue_idx].helix_id = Some(helix_id);
                info[residue_idx].sheet_id = None;
            }
        }

        self.assign_turns(&mut info, flags);
        info
    }

    fn assign_turns(&self, info: &mut [RibbonResidueInfo], flags: &[u32]) {
        for i in 0..info.len() {
            if info[i].ss != SecondaryStructure::Coil {
                continue;
            }

            if (flags[i]
                & (Self::DSSP_3DONOR
                    | Self::DSSP_3ACCEPTOR
                    | Self::DSSP_3GAP
                    | Self::DSSP_4DONOR
                    | Self::DSSP_4ACCEPTOR
                    | Self::DSSP_4GAP
                    | Self::DSSP_5DONOR
                    | Self::DSSP_5ACCEPTOR
                    | Self::DSSP_5GAP))
                != 0
            {
                info[i].ss = SecondaryStructure::Turn;
            }
        }
    }
}

fn calculate_imide_hydrogen(current: ResidueCoords, previous: ResidueCoords) -> Option<Vec3> {
    // BEGIN CHIMERAX CPP BODY: calculate_imide_hydrogen
    // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: add_imide_hydrogen
    // ChimeraX✔️✔️ CompSS.cpp:131 enters the missing-imide-H placement helper.
    // ChimeraX✔️✔️ CompSS.cpp:133-134 returns no new coordinate when H already exists.
    // ChimeraX✔️✔️ CompSS.cpp:136-139 copies current N/CA and previous C/O coordinates.
    // ChimeraX✔️✔️ CompSS.cpp:141-143 builds N-to-CA, N-to-previous-C, and previous-C-to-O vectors.
    // ChimeraX❗✔️ CompSS.cpp:144-146 normalizes vectors; Rust zero-protects degenerate vectors instead of throwing/asserting.
    // ChimeraX❗✔️ CompSS.cpp:147-150 normalizes the summed bisector/carbonyl direction with the same zero protection.
    // ChimeraX✔️✔️ CompSS.cpp:152 uses an N-H bond length of 1.01 A.
    // ChimeraX❌❌ CompSS.cpp:153 heap-allocates Coord; Rust returns an owned Vec3.
    // ChimeraX✔️✔️ CompSS.cpp:154 places H at N minus the normalized direction scaled by 1.01.
    // ChimeraX✔️✔️ CompSS.cpp:155 returns the synthesized coordinate.
    // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: add_imide_hydrogen
    // END CHIMERAX CPP BODY: calculate_imide_hydrogen
    let n_to_ca = (current.ca - current.n).normalize_or_zero();
    let n_to_c = (previous.c - current.n).normalize_or_zero();
    let c_to_o = (previous.o - previous.c).normalize_or_zero();
    let cac_bisect = (n_to_ca + n_to_c).normalize_or_zero();
    let opp_n = (cac_bisect + c_to_o).normalize_or_zero();

    (opp_n.length_squared() > 1e-6).then_some(current.n - opp_n * 1.01)
}

fn hbonded_to(acceptor: ResidueCoords, donor: ResidueCoords, cutoff: f32) -> bool {
    // BEGIN CHIMERAX CPP BODY: hbonded_to
    // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: hbonded_to
    // ChimeraX✔️✔️ CompSS.cpp:188 receives acceptor coordinates, donor coordinates, and cutoff.
    // ChimeraX✔️✔️ CompSS.cpp:190-192 uses electrostatic constants q1 = 0.42, q2 = 0.20, f = 332.
    // ChimeraX✔️✔️ CompSS.cpp:194-195 rejects donor residues with no H coordinate.
    // ChimeraX✔️✔️ CompSS.cpp:196-199 binds H, acceptor C/O, and donor N coordinates.
    // ChimeraX✔️✔️ CompSS.cpp:201 computes squared C-N distance.
    // ChimeraX✔️✔️ CompSS.cpp:202-203 rejects C-N distances above 7 A as an early cutoff.
    // ChimeraX✔️✔️ CompSS.cpp:204 converts C-N distance to linear distance.
    // ChimeraX✔️✔️ CompSS.cpp:205-207 computes O-N, C-H, and O-H distances.
    // ChimeraX❗✔️ CompSS.cpp:209 computes DSSP energy; Rust guards zero distances before division.
    // ChimeraX✔️✔️ CompSS.cpp:210 accepts the H-bond only when energy is below cutoff.
    // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: hbonded_to
    // END CHIMERAX CPP BODY: hbonded_to
    let Some(h) = donor.h else {
        return false;
    };

    let r_cn_sq = acceptor.c.distance_squared(donor.n);
    if r_cn_sq > 49.0 {
        return false;
    }

    let r_cn = r_cn_sq.sqrt();
    let r_on = acceptor.o.distance(donor.n);
    let r_ch = acceptor.c.distance(h);
    let r_oh = acceptor.o.distance(h);

    if r_cn <= 1e-6 || r_on <= 1e-6 || r_ch <= 1e-6 || r_oh <= 1e-6 {
        return false;
    }

    let energy = 0.42 * 0.20 * (1.0 / r_on + 1.0 / r_ch - 1.0 / r_oh - 1.0 / r_cn) * 332.0;
    energy < cutoff
}

fn peptide_connected(prev_c: Vec3, next_n: Vec3) -> bool {
    prev_c.distance_squared(next_n) <= 4.0
}

fn merge_bulge(lr1: BetaLadder, lr2: BetaLadder) -> Option<BetaLadder> {
    // BEGIN CHIMERAX CPP BODY: merge_bulge
    // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: merge_bulge
    // ChimeraX✔️✔️ CompSS.cpp:377 receives two ladder records.
    // ChimeraX✔️✔️ CompSS.cpp:379-380 creates mutable aliases for ordering.
    // ChimeraX✔️✔️ CompSS.cpp:381-382 rejects ladders of different type.
    // ChimeraX✔️✔️ CompSS.cpp:384-388 orders ladders by first-strand start position.
    // ChimeraX✔️✔️ CompSS.cpp:390-392 rejects first-strand gaps outside 0..=4.
    // ChimeraX✔️✔️ CompSS.cpp:393-397 computes second-strand gap using parallel or antiparallel geometry.
    // ChimeraX✔️✔️ CompSS.cpp:398-399 rejects second-strand gaps outside 0..=4.
    // ChimeraX✔️✔️ CompSS.cpp:400-401 rejects cases with more than one extra residue on both strands.
    // ChimeraX✔️✔️ CompSS.cpp:403-413 computes merged strand bounds by ladder orientation.
    // ChimeraX❌❌ CompSS.cpp:414 heap-allocates a ladder; Rust constructs the value directly.
    // ChimeraX✔️✔️ CompSS.cpp:415 marks the merged ladder as a beta bulge.
    // ChimeraX✔️✔️ CompSS.cpp:416 returns the merged ladder.
    // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: merge_bulge
    // END CHIMERAX CPP BODY: merge_bulge
    if lr1.is_bulge || lr2.is_bulge || lr1.ladder_type != lr2.ladder_type {
        return None;
    }

    let (l1, l2) = if lr1.start[0] <= lr2.start[0] {
        (lr1, lr2)
    } else {
        (lr2, lr1)
    };

    let d0 = l2.start[0] as isize - l1.end[0] as isize;
    if !(0..=4).contains(&d0) {
        return None;
    }

    let d1 = match l1.ladder_type {
        LadderType::Parallel => l2.start[1] as isize - l1.end[1] as isize,
        LadderType::Antiparallel => l1.start[1] as isize - l2.end[1] as isize,
    };
    if !(0..=4).contains(&d1) || (d0 > 1 && d1 > 1) {
        return None;
    }

    let (s1, e1) = match l1.ladder_type {
        LadderType::Parallel => (l1.start[1], l2.end[1]),
        LadderType::Antiparallel => (l2.start[1], l1.end[1]),
    };

    let mut ladder = BetaLadder::new(l1.ladder_type, l1.start[0], l2.end[0], s1, e1);
    ladder.is_bulge = true;
    Some(ladder)
}

fn find_beta_bulge(ladders: &mut Vec<BetaLadder>) -> bool {
    // BEGIN CHIMERAX CPP BODY: find_beta_bulge
    // BEGIN CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_beta_bulge
    // ChimeraX✔️✔️ CompSS.cpp:425 starts scanning the ladder list with the first iterator.
    // ChimeraX✔️✔️ CompSS.cpp:426 reads the first ladder.
    // ChimeraX✔️✔️ CompSS.cpp:427-428 skips first ladders that are already beta bulges.
    // ChimeraX✔️✔️ CompSS.cpp:429-430 scans later ladders as the second candidate.
    // ChimeraX✔️✔️ CompSS.cpp:431 reads the second ladder.
    // ChimeraX✔️✔️ CompSS.cpp:432-433 skips second ladders that are already beta bulges.
    // ChimeraX✔️✔️ CompSS.cpp:434 attempts to merge the pair.
    // ChimeraX✔️✔️ CompSS.cpp:435 enters the replacement path when merge succeeds.
    // ChimeraX✔️✔️ CompSS.cpp:436-438 removes both source ladders and appends the merged ladder.
    // ChimeraX❌❌ CompSS.cpp:439 deletes heap allocation; Rust has no explicit delete.
    // ChimeraX✔️✔️ CompSS.cpp:440 returns true after exactly one merge.
    // ChimeraX✔️✔️ CompSS.cpp:444 returns false when no eligible pair exists.
    // END CHIMERAX CPP FUNCTION: crates/core/src/parser/CompSS.cpp :: find_beta_bulge
    // END CHIMERAX CPP BODY: find_beta_bulge
    let mut i = 0;
    while i < ladders.len() {
        if ladders[i].is_bulge {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < ladders.len() {
            if ladders[j].is_bulge {
                j += 1;
                continue;
            }

            if let Some(merged) = merge_bulge(ladders[i], ladders[j]) {
                ladders.remove(j);
                ladders.remove(i);
                ladders.push(merged);
                return true;
            }

            j += 1;
        }

        i += 1;
    }

    false
}

fn sheet_groups_from_ladders(ladders: &[BetaLadder]) -> Vec<Vec<BetaLadder>> {
    let mut groups: Vec<Vec<BetaLadder>> = Vec::new();

    for &ladder in ladders {
        let mut group = vec![ladder];
        let mut idx = 0;
        while idx < groups.len() {
            if groups[idx]
                .iter()
                .any(|existing| ladders_share_residue(*existing, ladder))
            {
                group.extend(groups.remove(idx));
            } else {
                idx += 1;
            }
        }
        groups.push(group);
    }

    groups
}

fn ladders_share_residue(a: BetaLadder, b: BetaLadder) -> bool {
    ranges_overlap((a.start[0], a.end[0]), (b.start[0], b.end[0]))
        || ranges_overlap((a.start[0], a.end[0]), (b.start[1], b.end[1]))
        || ranges_overlap((a.start[1], a.end[1]), (b.start[0], b.end[0]))
        || ranges_overlap((a.start[1], a.end[1]), (b.start[1], b.end[1]))
}

fn ranges_overlap(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}

fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}
