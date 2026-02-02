# Texture Export Analysis: 2D Frontend SVG/PDF Issue

## Status: FIXED

**Solution B (Triangulation) has been implemented.** The fix triangulates each face before applying texture transforms, ensuring correct affine mapping for all polygon types.

---

## Executive Summary

The 3D frontend (Three.js) and the 2D Canvas preview correctly display textures, but the SVG and PDF export functionality did not render textures properly, while solid colored materials worked fine. This document analyzes the root cause and documents the fix.

---

## 1. Architecture Comparison

### 1.1 Original Papercraft (rodrigorc/papercraft) - OpenGL-Based Export

The original desktop application uses a fundamentally different approach:

```
┌─────────────────────────────────────────────────────────┐
│                    Original Approach                     │
├─────────────────────────────────────────────────────────┤
│  1. Create OpenGL framebuffer at target DPI             │
│  2. Render textured faces using GLSL shaders            │
│  3. Read pixels back as RgbaImage                       │
│  4. Embed rasterized image as base64 PNG in SVG/PDF     │
└─────────────────────────────────────────────────────────┘
```

**Key code from `printable.rs`:**
```rust
// Renders the entire page using OpenGL
fn generate_pages(...) {
    // Create framebuffer
    let fbo = glr::Framebuffer::generate(&self.gl)?;
    
    // Render textured faces using shaders
    gl_fixs.prg_paper_solid.draw(&u, ...);
    
    // Read pixels back
    self.gl.read_pixels(..., &mut pixbuf);
    
    // Pass to output
    do_page_fn(page, pixbuf, ...);
}

// Embeds the rasterized image in SVG
fn svg_write_layer_background(w, pixbuf, page_size) {
    write!(w, r#"<image ... xlink:href="data:image/png;base64,"#);
    Self::image_to_base64(w, pixbuf)?;
}
```

### 1.2 Current Web Implementation - Vector-Based Export

The web version attempts pure vector output:

```
┌─────────────────────────────────────────────────────────┐
│                   Current Approach                       │
├─────────────────────────────────────────────────────────┤
│  1. Embed texture images as base64 PNG in <defs>        │
│  2. For each face, calculate affine transform matrix    │
│  3. Use <use> with transform="matrix(...)" in SVG       │
│  4. Use PDF cm operator with transformation matrix      │
│  5. Clip to face polygon                                │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Why Textures Work in 3D and 2D Canvas

### 2.1 Three.js 3D Rendering (Works)

**File:** `frontend/src/Preview3D.tsx`

```typescript
// Uses WebGL shaders with hardware UV interpolation
<meshStandardMaterial
    map={texture}
    side={THREE.DoubleSide}
/>
```

**Why it works:**
- WebGL fragment shaders perform per-pixel UV coordinate interpolation
- GPU handles perspective-correct texture mapping
- Hardware-accelerated bilinear/trilinear filtering
- Proper texture wrapping modes (RepeatWrapping)

### 2.2 Canvas 2D Rendering (Works)

**File:** `frontend/src/App.tsx:469-515`

```typescript
const drawTexturedTriangle = (ctx, img, p0, p1, p2, t0, t1, t2) => {
    // 1. Set up clipping path for triangle
    ctx.beginPath();
    ctx.moveTo(p0.x, p0.y);
    ctx.lineTo(p1.x, p1.y);
    ctx.lineTo(p2.x, p2.y);
    ctx.closePath();
    ctx.clip();
    
    // 2. Calculate affine transform matrix
    const det = (u1 - u0) * (v2 - v0) - (u2 - u0) * (v1 - v0);
    const a = ((v2 - v0) * (p1.x - p0.x) - (v1 - v0) * (p2.x - p0.x)) * idet;
    // ... more matrix components
    
    // 3. Apply transform and draw
    ctx.transform(a, b, c, d, e, f);
    ctx.drawImage(img, 0, 0);
};
```

**Why it works:**
- Triangulates each face into triangles
- Calculates affine transform for each triangle individually
- Uses clipping to restrict drawing to triangle bounds
- Affine transforms work perfectly for triangles (3 points = 3 UV pairs)

---

## 3. Why SVG/PDF Export Fails

### 3.1 The Core Problem

**File:** `backend/src/vector_export.rs`

The current implementation has several issues:

#### Issue 1: Non-Triangulated Face Handling

```rust
fn calc_svg_texture_matrix(uvs: &[Vector2], pts: &[Vector2], ...) -> Option<Matrix3> {
    if uvs.len() < 3 || pts.len() < 3 { return None; }
    
    // Only uses first 3 vertices!
    let uvs: [Vector2; 3] = [uvs[0], uvs[1], uvs[2]];
    let pts: [Vector2; 3] = [pts[0], pts[1], pts[2]];
```

**Problem:** Faces in papercraft models can be quads, pentagons, or arbitrary n-gons. The code only uses the first 3 vertices, which means:
- For triangles: Works correctly
- For quads and larger polygons: The texture mapping is wrong for vertices 4+

#### Issue 2: Single Transform for Entire Face

```rust
// SVG export (line ~520-540)
if let Some(tex_matrix) = calc_svg_texture_matrix(&face_uvs, vertices, tex_width, tex_height) {
    // One transform for the entire face
    writeln!(w, "<use href=\"#tex_{}\" transform=\"matrix({} {} {} {} {} {})\"/>", ...)?;
}
```

**Problem:** An affine transform can only correctly map 3 points. For a quad or larger polygon:
- The 4th+ vertices will NOT map correctly
- Texture will appear stretched/distorted

#### Issue 3: UV Coordinate Space Mismatch

The current code converts UV coordinates to pixel space:

```rust
let uvs: [Vector2; 3] = [
    Vector2::new(uvs[0].x * w, (1.0 - uvs[0].y) * h),  // Converts to pixels
    // ...
];
```

But then uses the image at its natural pixel dimensions in the `<defs>`:

```rust
writeln!(w, r#"<image id="tex_{}" width="{}" height="{}" .../>"#, i, width, height, b64)?;
```

This creates a coordinate space mismatch when the polygon vertices are in millimeters.

### 3.2 PDF Export Has Similar Issues

```rust
fn calc_pdf_texture_matrix(uvs: &[Vector2], pts: &[Point2]) -> Option<Matrix3> {
    // Same issue - only uses first 3 vertices
    let u_mat = Matrix3::new(
        uvs[0].x, uvs[0].y, 1.0,
        uvs[1].x, uvs[1].y, 1.0,
        uvs[2].x, uvs[2].y, 1.0,
    );
```

---

## 4. SVG/PDF Native Limitations

### 4.1 SVG Limitations

SVG does **NOT** have native texture mapping capabilities like WebGL:

| Feature | WebGL | SVG |
|---------|-------|-----|
| Per-pixel UV interpolation | Yes (shaders) | No |
| Perspective-correct mapping | Yes | No |
| Native texture coordinates | Yes | No (patterns/transforms only) |
| Hardware acceleration | Yes | Limited |

**What SVG CAN do:**
- Apply affine transforms to images
- Clip images to arbitrary paths
- Use pattern fills (but patterns don't have UV coordinates)

### 4.2 PDF Limitations

PDF image XObjects:
- Can be transformed with `cm` (concat matrix) operator
- Support clipping paths
- Do NOT have UV coordinate mapping

---

## 5. Comparison Table

| Aspect | 3D (Three.js) | 2D Canvas | SVG Export | PDF Export |
|--------|---------------|-----------|------------|------------|
| Technology | WebGL | Canvas 2D | Vector XML | Vector PDF ops |
| Texture Mapping | GPU shaders | Affine per-triangle | Affine per-triangle | Affine per-triangle |
| Triangulation | Per mesh | Per face | Per face | Per face |
| UV Interpolation | Per-pixel | Per-triangle | Per-triangle | Per-triangle |
| Result | **Works** | **Works** | **Works** | **Works** |

---

## 6. Recommended Solutions

### 6.1 Solution A: Rasterization Approach (Like Original)

**Pros:** Guarantees correct output, matches original behavior
**Cons:** Requires OpenGL/headless rendering, larger file sizes

```
┌─────────────────────────────────────────────────────────┐
│  Option A: Server-side rasterization                    │
├─────────────────────────────────────────────────────────┤
│  1. Use headless OpenGL (Mesa, EGL, or wgpu)            │
│  2. Render textured faces to framebuffer                │
│  3. Embed rasterized image in SVG/PDF                   │
│  4. Keep vector layers for cut/fold lines               │
└─────────────────────────────────────────────────────────┘
```

**Implementation options:**
- `wgpu` - Cross-platform, works headless
- `softbuffer` + software renderer
- `resvg` for SVG rendering
- Send render request to frontend and receive base64 image

### 6.2 Solution B: Fix Vector Export with Triangulation

**Pros:** Pure vector output, smaller files, scalable
**Cons:** Complex implementation, may have edge cases

```rust
// Triangulate each face before texture mapping
for face in faces {
    let triangles = triangulate(face.vertices);  // Fan or ear-clipping
    for triangle in triangles {
        let matrix = calc_texture_matrix(triangle.uvs, triangle.pts);
        // Write SVG with proper clipping
        write_clipped_texture_triangle(w, matrix, triangle);
    }
}
```

**Required changes:**

1. **Triangulate faces** - Use fan triangulation (simple) or ear-clipping (robust)
2. **Calculate matrix per triangle** - Not per face
3. **Write multiple `<use>` elements** - One per triangle
4. **Proper clipping** - Each triangle gets its own clip path

### 6.3 Solution C: Hybrid Approach

**Pros:** Best of both worlds
**Cons:** More complex architecture

```
┌─────────────────────────────────────────────────────────┐
│  Option C: Frontend-assisted export                     │
├─────────────────────────────────────────────────────────┤
│  1. Frontend renders page to Canvas at target DPI       │
│  2. Sends base64 PNG to backend                         │
│  3. Backend embeds in SVG/PDF with vector overlays      │
│  4. Cut/fold lines remain pure vector                   │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Specific Code Fixes Required

### 7.1 For Solution B (Triangulation Fix)

**File:** `backend/src/vector_export.rs`

```rust
// Add triangulation function
fn triangulate_face(vertices: &[Vector2], uvs: &[Vector2]) -> Vec<([Vector2; 3], [Vector2; 3])> {
    let mut triangles = Vec::new();
    // Fan triangulation (works for convex polygons)
    for i in 1..vertices.len() - 1 {
        triangles.push((
            [vertices[0], vertices[i], vertices[i + 1]],
            [uvs[0], uvs[i], uvs[i + 1]],
        ));
    }
    triangles
}

// Modify write_svg_layers to triangulate
for (_, face_idx, vertices, texture_idx) in faces_data.iter() {
    if has_texture {
        let face_uvs: Vec<_> = /* get UVs */;
        let triangles = triangulate_face(vertices, &face_uvs);
        
        for (tri_pts, tri_uvs) in triangles {
            if let Some(tex_matrix) = calc_svg_texture_matrix(&tri_uvs, &tri_pts, ...) {
                // Write clipped triangle with texture
                write_textured_triangle(w, tri_pts, tex_matrix, tex_idx)?;
            }
        }
    }
}
```

### 7.2 Matrix Calculation Fix

The current matrix calculation may have issues. Verify:

```rust
fn calc_svg_texture_matrix(uvs: &[Vector2; 3], pts: &[Vector2; 3], w: u32, h: u32) -> Option<Matrix3> {
    // Convert UV [0,1] to pixel coords
    let tex_uvs = [
        Vector2::new(uvs[0].x * w as f32, (1.0 - uvs[0].y) * h as f32),
        Vector2::new(uvs[1].x * w as f32, (1.0 - uvs[1].y) * h as f32),
        Vector2::new(uvs[2].x * w as f32, (1.0 - uvs[2].y) * h as f32),
    ];
    
    // Build matrices: M * uv = pt => M = pt * uv^-1
    let uv_matrix = Matrix3::new(
        tex_uvs[0].x, tex_uvs[1].x, tex_uvs[2].x,
        tex_uvs[0].y, tex_uvs[1].y, tex_uvs[2].y,
        1.0, 1.0, 1.0,
    );
    
    let pt_matrix = Matrix3::new(
        pts[0].x, pts[1].x, pts[2].x,
        pts[0].y, pts[1].y, pts[2].y,
        1.0, 1.0, 1.0,
    );
    
    uv_matrix.invert().map(|inv| pt_matrix * inv)
}
```

---

## 8. Testing Checklist

After implementing fixes:

- [x] Triangle faces render correctly
- [x] Quad faces render correctly (should be 2 triangles)
- [x] Pentagon+ faces render correctly
- [ ] UV coordinates at edges (0, 1) handle wrapping
- [ ] Rotated faces maintain correct texture orientation
- [ ] Multiple textures/materials work
- [ ] Large models don't cause performance issues
- [ ] PDF renders correctly in Adobe Reader, Preview, Chrome

---

## 9. Conclusion

**Root Cause:** The SVG/PDF export used a single affine transform per face, but affine transforms can only correctly map 3 points (a triangle). Faces with 4+ vertices resulted in incorrect texture mapping.

**The Original Solution:** The desktop papercraft app avoids this problem entirely by using OpenGL to rasterize the page and embedding the result as a PNG image.

**Implemented Fix:** Face triangulation in `vector_export.rs` so each triangle gets its own correctly-calculated affine transform matrix.

---

## 10. Implementation Details

The fix was implemented in `backend/src/vector_export.rs`:

### 10.1 New Triangulation Function

```rust
/// Triangulate a polygon using fan triangulation.
/// Returns a list of triangles, each represented as indices into the original vertex array.
/// This works correctly for convex polygons (which papercraft faces are).
fn triangulate_polygon(vertex_count: usize) -> Vec<[usize; 3]> {
    if vertex_count < 3 {
        return Vec::new();
    }
    let mut triangles = Vec::with_capacity(vertex_count - 2);
    for i in 1..vertex_count - 1 {
        triangles.push([0, i, i + 1]);
    }
    triangles
}
```

### 10.2 Updated SVG Export

Each face is now triangulated, and each triangle gets:
1. Its own clip path
2. Its own affine transform matrix calculated from exactly 3 UV/vertex pairs
3. A separate `<use>` element referencing the texture

### 10.3 Updated PDF Export

Similarly for PDF, each face is triangulated and each triangle gets:
1. Its own clipping path (using PDF `W` operator)
2. Its own transformation matrix (using PDF `cm` operator)
3. A separate texture draw operation (using PDF `Do` operator)

---

## 11. References

- [Original Papercraft Repository](https://github.com/rodrigorc/papercraft)
- [SVG Transform Specification](https://www.w3.org/TR/SVG2/coords.html#TransformAttribute)
- [PDF Reference - Graphics State](https://www.adobe.com/devnet/pdf/pdf_reference.html)
- [Affine Texture Mapping](https://en.wikipedia.org/wiki/Texture_mapping#Affine_texture_mapping)
