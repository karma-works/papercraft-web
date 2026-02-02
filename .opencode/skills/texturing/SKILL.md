---
name: texturing
description: |
  Specialized knowledge for implementing texture mapping in 2D vector exports (SVG/PDF) for the papercraft web application. Use this skill when working on: (1) SVG or PDF texture export issues, (2) Affine texture mapping problems, (3) Face triangulation for correct UV mapping, (4) Debugging texture rendering differences between 3D preview and 2D exports, (5) Understanding the texture pipeline from 3D model to printable output.
---

# Texturing Skill for Papercraft

Provides domain knowledge for texture mapping in the papercraft web application's vector export pipeline.

## Codebase Map

### Backend (Rust) - Vector Export

| File | Line | Function | Purpose |
|------|------|----------|---------|
| `backend/src/vector_export.rs` | 48 | `generate_svg()` | SVG single-page export entry point |
| `backend/src/vector_export.rs` | 55 | `generate_svg_multipage()` | SVG multi-page export entry point |
| `backend/src/vector_export.rs` | 168 | `write_svg_defs()` | Embeds textures as base64 PNG in `<defs>` |
| `backend/src/vector_export.rs` | 198 | `triangulate_polygon()` | Fan triangulation for n-gons |
| `backend/src/vector_export.rs` | 211 | `calc_texture_matrix()` | Core affine matrix calculation |
| `backend/src/vector_export.rs` | 238 | `calc_svg_texture_matrix_triangle()` | UV-to-pixel coordinate conversion for SVG |
| `backend/src/vector_export.rs` | 263 | `write_svg_layers()` | Writes faces with per-triangle texture mapping |
| `backend/src/vector_export.rs` | 797 | `calc_pdf_texture_matrix_triangle()` | Matrix calculation for PDF export |
| `backend/src/vector_export.rs` | 868 | `generate_pdf()` | PDF export entry point |
| `backend/src/vector_export.rs` | 992 | `generate_pdf_page_ops()` | PDF operations with per-triangle textures |

### Backend (Rust) - Model & Textures

| File | Line | Item | Purpose |
|------|------|------|---------|
| `backend/src/paper/model.rs` | 16 | `struct Texture` | Texture data structure |
| `backend/src/paper/model.rs` | 39 | `Texture::pixbuf()` | Access texture pixel data |
| `backend/src/paper/model.rs` | 586 | `Vertex::uv()` | Get UV coordinates for a vertex |
| `backend/src/main.rs` | 199 | `/api/texture/:id` | HTTP endpoint serving texture images |

### Frontend (TypeScript/React)

| File | Line | Function | Purpose |
|------|------|----------|---------|
| `frontend/src/App.tsx` | 365 | Texture loading effect | Loads textures from `/api/texture/:id` |
| `frontend/src/App.tsx` | 469 | `drawTexturedTriangle()` | Canvas 2D per-triangle texture mapping |
| `frontend/src/App.tsx` | 1032 | Texture draw call | Applies texture to each face triangle |
| `frontend/src/Preview3D.tsx` | 65 | Texture assignment | Three.js material texture loading |

### Tests

| File | Line | Test | Purpose |
|------|------|------|---------|
| `backend/src/svg_tests.rs` | 62 | `test_sphere_pdo_svg_export_with_textures` | Validates textured SVG export |
| `backend/src/svg_tests.rs` | 73 | Texture data check | Verifies textures have pixel data |

## Core Algorithm: Triangulation

Affine transforms map exactly 3 points. For n-gon faces, triangulate first:

```rust
// backend/src/vector_export.rs:198
fn triangulate_polygon(vertex_count: usize) -> Vec<[usize; 3]> {
    let mut triangles = Vec::with_capacity(vertex_count - 2);
    for i in 1..vertex_count - 1 {
        triangles.push([0, i, i + 1]);
    }
    triangles
}
```

**Usage pattern** (SVG export at line ~500):
```rust
let triangles = triangulate_polygon(vertices.len());
for (tri_idx, tri_indices) in triangles.iter().enumerate() {
    let tri_pts = [vertices[i0], vertices[i1], vertices[i2]];
    let tri_uvs = [face_uvs[i0], face_uvs[i1], face_uvs[i2]];
    if let Some(matrix) = calc_svg_texture_matrix_triangle(tri_uvs, tri_pts, w, h) {
        // Write clip path + transformed texture for this triangle
    }
}
```

## UV Coordinate Handling

**Critical**: PDO format uses V=0 at top; images use V=0 at bottom.

```rust
// backend/src/vector_export.rs:238-258
fn calc_svg_texture_matrix_triangle(uvs: [Vector2; 3], pts: [Vector2; 3], ...) {
    let pixel_uvs = [
        Vector2::new(uvs[0].x * w, (1.0 - uvs[0].y) * h),  // Flip V!
        // ...
    ];
    calc_texture_matrix(pixel_uvs, pts)
}
```

## SVG Output Structure

Generated at `write_svg_layers()` (line 263):

```xml
<g inkscape:label="Faces">
  <!-- Per triangle -->
  <defs><clipPath id="clip_face_0_0"><polygon points="..."/></clipPath></defs>
  <g clip-path="url(#clip_face_0_0)">
    <use href="#tex_0" transform="matrix(a b c d e f)"/>
  </g>
</g>
```

## PDF Output Structure

Generated at `generate_pdf_page_ops()` (line 992):

```
q                    % Save state
m x0 y0              % Move to first point
l x1 y1              % Line to second
l x2 y2              % Line to third  
h                    % Close path
W                    % Set clipping path
n                    % End path
cm a b c d e f       % Transformation matrix
Do /Im0              % Draw texture
Q                    % Restore state
```

## Coordinate Systems

| System | Origin | Y Direction | Units |
|--------|--------|-------------|-------|
| UV | Bottom-left | Up | 0-1 |
| Texture pixels | Top-left | Down | pixels |
| Model | Page corner | Down | mm |
| SVG | Top-left | Down | mm (via viewBox) |
| PDF | Bottom-left | Up | points (72/inch) |

**Conversions**:
```rust
// mm to points
let pt = mm * 72.0 / 25.4;

// SVG Y to PDF Y  
let pdf_y = page_height_mm - svg_y;
```

## Debugging Workflow

1. **Check texture data exists**:
   ```rust
   // backend/src/paper/model.rs:463
   model.textures().any(|t| t.pixbuf().is_some())
   ```

2. **Verify UV coordinates**:
   ```rust
   // backend/src/paper/model.rs:586
   let uv = vertex.uv();  // Should be in [0,1] range
   ```

3. **Test matrix calculation**:
   ```bash
   cd backend && cargo test test_texture_matrix
   ```

4. **Compare Canvas 2D output**:
   - Canvas 2D at `App.tsx:469` uses same algorithm
   - If Canvas works but SVG doesn't, check coordinate conversion

## Common Issues

| Symptom | Cause | Fix Location |
|---------|-------|--------------|
| Textures work in 3D, broken in export | Single transform per face | `vector_export.rs` - use `triangulate_polygon()` |
| Textures mirrored | V-flip missing | `calc_svg_texture_matrix_triangle()` line 248 |
| Textures offset | Wrong coordinate space | Check `write_svg_defs()` image dimensions |
| Crash on some faces | Degenerate triangle | Handle `None` from `calc_texture_matrix()` |

## API Endpoints

```bash
# Get texture image
GET /api/texture/:id

# Export with textures
GET /api/export?format=svg&textures=true
GET /api/export?format=pdf&textures=true
```

## References

- `TEXTURE_EXPORT_ANALYSIS.md` - Full analysis in project root
- `references/implementation.md` - Detailed algorithm explanations
- `references/fundamentals.md` - fundamentals of the pdo format
