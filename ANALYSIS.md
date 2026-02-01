# Analysis of Texture Export Issues

## Issues Identified
1. **SVG Export**: 
   - Texture transforms were incorrect, resulting in textures not matching the face polygons.
   - The matrix calculation used `.transpose()` which was flipping the matrix logic incorrectly for cgmath's column-major structure.
   - Textures were defined with `width="1" height="1"` in earlier exports, but this seems resolved in current build.
   - The `calc_texture_matrix` function was constructing matrices incorrectly.

2. **PDF Export**:
   - Similar matrix calculation issues as SVG.
   - PDF texture embedding and referencing logic needs verification.
   - Empty pages (2 & 3) issue reported.

## Attempts to Solve
1. **Analysis of Export Files**: 
   - Examined `export.svg` vs `export3.svg` vs `export4.svg`.
   - Identified that `export3.svg` had `width="1"` issue.
   - `export4.svg` had correct dimensions but wrong transform matrix (tiny translation values).

2. **Diagnostic Tests**:
   - Created failing unit tests in `svg_tests.rs`.
   - `test_texture_transform_matrix_maps_uvs_to_vertices`: Confirmed matrix calculation was wrong.
   - `test_pdf_texture_matrix_calculation`: Confirmed PDF matrix calculation was also wrong.
   - `test_svg_texture_transform_places_texture_at_face_location`: Failed initially due to regex issues, but useful concept.

3. **Matrix Logic Verification**:
   - Created a "manual" matrix verification test `test_pdf_texture_matrix_calculation` that constructs the matrix mathematically correctly.
   - Proved that removing `.transpose()` from the matrix construction yields the correct result.
   - cgmath constructs matrices column-by-column: `Matrix3::new(c0r0, c0r1, c0r2, c1r0, ...)`
   - The code was calling `Matrix3::new(...)` passing values in row order, then calling `.transpose()`, which effectively created a matrix where columns were rows.
   - But we want columns to be the [x, y, 1] vectors.
   - Correct approach: `Matrix3::new(v0.x, v0.y, 1.0, v1.x, v1.y, 1.0, v2.x, v2.y, 1.0)` constructs a matrix where columns are v0, v1, v2. This is exactly what we want for U and P matrices.
   - The existing code did `Matrix3::new(v0.x, v0.y, 1.0, ...).transpose()`. This took the column-major matrix (which was already correct!) and transposed it, swapping rows/columns incorrectly.

## Untried Approaches
1. **Fixing the Code**: I haven't applied the fix to `vector_export.rs` yet.
2. **Verifying PDF Empty Pages**: I haven't deeply investigated why pages 2 & 3 are empty in PDF export yet. This might be related to coordinate system transforms or page offset handling.
3. **PDF Texture Rendering**: The "black areas" in PDF might be due to inverted Y-axis or incorrect culling/clipping, or simply the wrong matrix making the texture disjoint from the clip path.

## Next Steps
1. Fix `calc_texture_matrix` in `vector_export.rs`.
2. Fix `calc_pdf_texture_matrix` in `vector_export.rs`.
3. Re-run tests to confirm fixes.
4. Investigate PDF empty pages issue (likely coordinate transform related).
