# Analysis: PDF Coordinate Offset Issue
**Date:** 2026-01-18

## Context
Elements in the PDF export (islands, fold lines, flaps) are being placed outside the visible page boundaries, while they appear correctly in the Web UI and SVG export. This suggests that the relative coordinate transformation in the PDF backend is adding an incorrect offset or failing to account for the internal coordinate system's layout.

## Findings
- **Relevant Files:**
  - `backend/src/vector_export.rs`: Implementation of `generate_pdf_page_ops`.
  - `backend/src/paper/craft.rs`: Implementation of `is_in_page_fn` and `page_position`.

- **Coordinate Systems:**
  - **Internal/SVG:** The project uses a global "canvas" where pages are arranged in a grid separated by `PAGE_SEP` (10.0mm). 
  - **SVG Relativization:** `transformed - page_offset` explicitly subtracts the global top-left corner of the page from every point.
  - **PDF Relativization:** The current PDF implementation uses `in_page(p)`, which calls `options.is_in_page_fn(page)`. 
  - **The Error:** In `backend/src/paper/craft.rs` (line 365), `is_in_page_fn` calculates `r = p - page_pos_0`. This is the relative coordinate *within the page slot*. However, the islands are already stored with global coordinates that include page offsets.

- **Double Offset Risk:** 
  - If the PDF iteration uses `in_page(p)` on a point that was already calculated using an island matrix which contains the page offset, and `in_page` subtracts that offset again, it might work—but only if the `page` argument matches the page the island was packed into.
  - **Actual Logic Flaw:** The `in_page` function returns `(is_in, relative_pos)`. If `is_in` is false, the element is skipped. But for islands spanning multiple pages or near boundaries, the `is_in` check might be failing or the relative coordinate might be shifted incorrectly if the PDF viewer interprets the `MediaBox` differently than the internal layout.

- **PDF Y-Inversion Bug:**
  - In `backend/src/vector_export.rs`: `let pdf_y = |y: f32| (page_size_mm.y - y) * 72.0 / 25.4;`
  - This function assumes `y` is already relative to the top of the page (0 to page_height).
  - If `in_page(p)` returns a coordinate where `y` is negative or greater than `page_height`, `pdf_y` will produce coordinates outside the `MediaBox`.

## Potential Bottlenecks
- **Packing vs Export:** `pack_islands` (line 1502) places islands at `zero + pos`, where `zero` includes the `page_position`. This means the island's `loc` is in global canvas space.
- **Filtering Logic:** The PDF loop iterates over *all* islands for *every* page and checks `in_page` for every vertex. This is correct for filtering but inefficient and prone to coordinate drift if the `in_page` relative vector isn't used strictly.
- **Page Margins:** `pack_islands` uses `page_margin`, but `write_svg_layers` and `generate_pdf_page_ops` do not seem to explicitly account for margins when relativizing, potentially pushing elements into the "bleed" area.
