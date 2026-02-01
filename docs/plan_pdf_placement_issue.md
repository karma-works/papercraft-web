# Plan: PDF Coordinate Alignment Fix
**Reference Analysis:** docs/analysis_pdf_placement_issue.md

## Implementation Strategy
Standardize the PDF coordinate relativization logic to match the SVG implementation. Instead of relying on per-point `in_page` filtering which can lead to drift and clipping issues, we will calculate a fixed `page_offset` for the entire page and subtract it from all global coordinates before applying the PDF Y-inversion and PT-scaling.

## Detailed Changes

### 1. Standardize Offset Calculation
In `generate_pdf_page_ops` (`backend/src/vector_export.rs`):
- Calculate `let page_offset = options.page_position(page);` at the start of the function.
- This ensures we use the exact same grid reference as the SVG export.

### 2. Update Geometry Loops (Faces, Folds, Flaps, Cut Lines)
For every drawing operation:
- **Transform:** `let p_local = p_global - page_offset;`
- **Y-Inversion:** Use `pdf_y(p_local.y)` where `pdf_y` is relative to the page top.
- **Filtering:** Use a simple bounding box check or the `options.global_to_page(center)` logic to skip islands that aren't on the current page, rather than checking every single vertex with `in_page`.

### 3. Refactor `pdf_y` and `mm_to_pt` usage
Ensure that `mm_to_pt` is always applied to coordinates that have already been shifted into the `[0..page_width]` and `[0..page_height]` range.

### 4. Fix Text and Page Numbers
- Ensure `collect_texts` results are shifted by the same `page_offset` logic.
- Verify that footer elements (Page X/Y) are within the `MediaBox`.

## Verification Plan
1. **Compile:** `cargo check -p papercraft-backend`.
2. **Coordinate Trace:** Print the `page_offset` and the first vertex of a known island during PDF generation and compare it to the SVG output.
3. **Visual Regression:** Export a multi-page PDF and verify that islands on Page 2+ are no longer shifted off-canvas.
