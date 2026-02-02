# Plan: PDF Export Parity Fix
**Reference Analysis:** docs/analysis_pdf_island_issue.md

## Implementation Strategy
Achieve parity between PDF and SVG exports by porting the missing rendering layers (Folds and Flaps) from the SVG logic to the PDF logic in `backend/src/vector_export.rs`.

## Detailed Changes

### 1. Implement Fold Lines in `generate_pdf_page_ops`
- **Logic:** Iterate over island faces and edges (similar to `write_svg_layers` line 222).
- **Filtering:** Only process edges with `EdgeStatus::Joined`.
- **Styling:** 
  - Mountain: Solid line (black).
  - Valley: Dashed line (black).
- **Scaling:** Apply `in_page` and then `mm_to_pt` / `pdf_y`.

### 2. Implement Flaps in `generate_pdf_page_ops`
- **Logic:** Iterate over island perimeter edges (similar to `write_svg_layers` line 258).
- **Geometry:** Calculate flap vertices `[p0, p1, f1, f0]`.
- **Rendering:** 
  - Fill with light gray (`0.88, 0.88, 0.88`).
  - Stroke with tab color.

### 3. Fix Text Alignment in `generate_pdf_page_ops`
- **Logic:** Adjust the `x` coordinate of the text based on `text.align`.
- **Calculation:**
  - `Center`: Subtract `(text_width / 2)`. (Approximate width using font size since full layout isn't available).
  - `Far`: Subtract `text_width`.
- **Note:** Since `lopdf` doesn't provide text metrics, we will use a heuristic: `width = char_count * size * 0.6`.

### 4. Code Structure
- Refactor the shared data collection logic (Folds/Flaps) into helper functions if possible, or duplicate for now to ensure strict compliance with PDF `Operation` API which differs significantly from SVG `writeln!`.

## Verification Plan
1. **Compile:** `cargo check -p papercraft-backend`.
2. **Visual Inspection:** Generate a PDF from a known model (e.g., `pikachu.craft`) and verify that fold lines and tabs are now visible.
3. **Alignment Test:** Check that Page numbers and Island labels are correctly aligned.
