A ``MoleculeColors`` object holds element colors and optional atom colors. Use
element symbols such as ``C`` and ``O`` as keys. Omitted elements use the viewer's
built-in colors.

- **Scene palette.** Set the palette before adding molecules or replacing
  displays. It supplies the element colors for molecular displays that use
  the scene palette.
- **Display colors.** Pass a scheme through a molecular style's ``colors()``
  method. That scheme supplies its own element palette. Derive atom colors
  from a base palette to keep its element colors, and reuse the resulting
  scheme or style on as many displays as needed.
- **Atom IDs.** Use zero-based CK atom IDs from the original molecule, including
  when displaying a selection. A color applies to the atom's sphere and to
  the half of each bond next to that atom. An explicit atom color takes
  precedence over the style's single color. Otherwise the viewer uses the
  single color when set, any applicable source color, then the element color.
- **Restore colors.** Obtain the built-in scheme with
  ``MoleculeColors.default()``. Apply it to one style to reset that display, or
  set it as the scene palette to reset the defaults for later displays.
  To recolor existing displays, replace them by name with ``set`` and update
  the viewer if the scene is already open.
