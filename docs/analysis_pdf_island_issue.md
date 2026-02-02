# Analysis: PDF vs SVG Island Display Discrepancy
**Date:** 2026-01-18

## Context
The user reports that 2D islands are correctly displayed in SVG exports but are incorrect or missing in PDF exports. A preliminary investigation suggested a coordinate system mismatch (absolute vs. relative).

## Findings
- **Relevant Files:**
  - `backend/src/vector_export.rs`: Contains the primary logic for both SVG (`write_svg_layers`) and PDF (`generate_pdf_page_ops`) generation.
  - `backend/src/paper/craft.rs`: Defines the `PaperOptions` and `in_page_fn` logic used to transform coordinates.

- **Logic Flow:**
  - **SVG:** The SVG export uses `page_offset` calculated via `options.page_position(page)` and subtracts it from global coordinates (`transformed - page_offset`). This creates a local coordinate system relative to the SVG canvas.
  - **PDF:** The PDF export uses `options.is_in_page_fn(page)`. This function returns a tuple `(bool, Vector2)`. The second element is the coordinate relative to the page start.
  - **The Bug:** Previously, the PDF loop for drawing faces checked `in_page(*v).0` (is in page) but ignored the transformed coordinate, using global coordinates instead. This was partially fixed in a previous turn for faces, but other elements (like cut lines, flaps, or text) might still be inconsistent.

## Potential Bottlenecks
- **Coordinate Inversion:** PDF uses a bottom-up Y-coordinate system (standard PDF spec), while SVG and the internal model use top-down. The `pdf_y` helper `(page_size_mm.y - y) * 72.0 / 25.4` handles this, but it must be applied to *already relativized* coordinates.
- **Flaps and Cut Lines:** The `generate_pdf_page_ops` function iterates through islands and perimeters. If `island_mx` or face matrices are applied without accounting for the page-relative shift provided by `in_page`, the lines will be drawn at global offsets (e.g., if an island is on Page 2, it might be drawn 300mm off-canvas).
- **Scale Consistency:** Ensure `mm_to_pt` (72/25.4) is applied uniformly after all relative transformations are done.
