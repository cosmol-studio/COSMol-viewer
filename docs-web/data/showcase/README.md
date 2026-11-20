# Showcase datasets

- The homepage structure is the human vitamin D receptor bound to calcitriol
  ([PDB 1DB1](https://www.rcsb.org/structure/1DB1)). The protein file retains
  the deposited chain A coordinates. The ligand file is the
  [VDX instance from the RCSB ModelServer](https://models.rcsb.org/v1/1db1/ligand?auth_asym_id=A&auth_seq_id=428&label_comp_id=VDX&encoding=sdf).
  The receptor and ligand retain their relative placement from the crystal structure.
- `1iep-receptor.pdb` retains the deposited protein atoms from chain A of
  [PDB 1IEP](https://www.rcsb.org/structure/1IEP), which contains the reference ligand.
- `imatinib-poses.sdf` contains the crystal ligand and the four saved Vina
  poses from the [official basic docking example](https://autodock-vina.readthedocs.io/en/stable/docking_basic.html).
  The Vina PDBQT atom coordinates were exported to SDF using the stored
  `REMARK SMILES IDX` mapping and SMILES bond orders. Displayed scores are copied
  from the corresponding `REMARK VINA RESULT` records, in kcal/mol.
  The viewer kekulizes the ligands on load so the sticks show alternating bond
  orders without moving the atoms.
- The conformer demo uses sequences 000, 001 and 005 from the
  [source frame archive](https://obj.cosmol.org/cosmol/2025_12_03_0039_frames.tar.gz).
  Each file `conformers-NNN.txt` has 50 rows in source order. Each row stores
  the unchanged SDF XYZ values for all 43 atoms, in the shared atom order of
  `conformer-topology.sdf` (frame 000 of sequence 000). Atom and bond blocks
  were checked for consistency across the three sequences. The playback interval
  is for visualization and does not represent simulation time.
- The interface example uses [PDB 1VFB](https://www.rcsb.org/structure/1VFB).
  The two PDB files retain the deposited coordinates for antibody chains A/B and
  lysozyme chain C. The viewer marks residues with at least one inter-partner
  heavy-atom distance of 4 Å or less.
- The structure comparison uses chain A from [4AKE](https://www.rcsb.org/structure/4AKE)
  (open) and [1AKE](https://www.rcsb.org/structure/1AKE) (closed). The stored 1AKE
  coordinates were rigidly fitted onto 4AKE by the corresponding Cα atoms of
  residues 1–29, 68–117 and 161–214. This used 133 pairs, a proper rotation, and
  gave a fitted Cα RMSD of 1.551 Å before PDB serialization.
