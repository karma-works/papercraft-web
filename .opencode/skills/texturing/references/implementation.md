# Implementation Reference

## Table of Contents
1. [Code Locations Quick Reference](#code-locations-quick-reference)
2. [Triangulation Algorithm](#triangulation-algorithm)
3. [SVG Export Deep Dive](#svg-export-deep-dive)
4. [PDF Export Deep Dive](#pdf-export-deep-dive)
5. [Matrix Mathematics](#matrix-mathematics)
6. [Frontend Texture Handling](#frontend-texture-handling)

---

## Code Locations Quick Reference

### Critical Functions (modify these for texture behavior)

```
backend/src/vector_export.rs:
  :198  triangulate_polygon()        - Splits faces into triangles
  :211  calc_texture_matrix()        - Core 3-point affine math
  :238  calc_svg_texture_matrix_triangle() - SVG-specific UV conversion
  :263  write_svg_layers()           - Main SVG face rendering loop
  :797  calc_pdf_texture_matrix_triangle() - PDF-specific matrix
  :992  generate_pdf_page_ops()      - Main PDF face rendering loop

backend/src/paper/model.rs:
  :16   struct Texture               - Texture data holder
  :39   pixbuf()                     - Get texture pixel data
  :586  uv()                         - Get vertex UV coordinates
```

### Data Flow

```
Model Import (PDO/OBJ/glTF)
    │
    ├── backend/src/paper/model/import/pepakura/importer.rs:116
    │   └── Extracts texture pixbuf from PDO
    │
    ├── backend/src/paper/model/import/waveobj/importer.rs:138
    │   └── Loads texture from MTL reference
    │
    └── backend/src/paper/model/import/gltf/importer.rs:87
        └── Extracts embedded textures from glTF
            │
            ▼
    backend/src/paper/model.rs:16 (struct Texture)
            │
            ▼
    ┌───────┴───────┐
    │               │
    ▼               ▼
  /api/texture    vector_export.rs
  (main.rs:199)   (write_svg_defs:168)
```

---

## Triangulation Algorithm

### Location
`backend/src/vector_export.rs:198-209`

### Implementation
```rust
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

### Visual Explanation
```
Quad [0,1,2,3]:          Pentagon [0,1,2,3,4]:
    
   0─────1                  0─────1
   │\    │                 /│\    │
   │ \ T0│                / │ \T0 │
   │  \  │               /  │  \  │
   │T1 \ │              /T2 │T1 \ │
   │    \│             /    │    \│
   3─────2            4─────3─────2

T0 = [0,1,2]            T0 = [0,1,2]
T1 = [0,2,3]            T1 = [0,2,3]
                        T2 = [0,3,4]
```

### Why This Works
- Papercraft faces are convex (from 3D mesh triangulation)
- Fan triangulation is O(n) complexity
- Indices map directly to UV array

---

## SVG Export Deep Dive

### Entry Points
```
generate_svg()           @ vector_export.rs:48
generate_svg_multipage() @ vector_export.rs:55
```

### Texture Embedding
```
write_svg_defs() @ vector_export.rs:168-193

For each texture with pixbuf:
1. Get dimensions (width, height)
2. Encode as PNG to buffer
3. Base64 encode
4. Write: <image id="tex_{i}" width="{w}" height="{h}" href="data:image/png;base64,{b64}"/>
```

### Face Rendering Loop
```
write_svg_layers() @ vector_export.rs:263-600

For each face with texture:
    face_uvs = get UV coordinates from model
    triangles = triangulate_polygon(vertices.len())
    
    For each triangle:
        tri_pts = [vertices[i0], vertices[i1], vertices[i2]]
        tri_uvs = [face_uvs[i0], face_uvs[i1], face_uvs[i2]]
        
        matrix = calc_svg_texture_matrix_triangle(tri_uvs, tri_pts, tex_w, tex_h)
        
        Write clip path:
            <clipPath id="clip_face_{idx}_{tri_idx}">
              <polygon points="{tri_pts}"/>
            </clipPath>
        
        Write texture reference:
            <g clip-path="url(#clip_face_{idx}_{tri_idx})">
              <use href="#tex_{tex_idx}" transform="matrix({matrix})"/>
            </g>
```

### Matrix Extraction for SVG
```rust
// vector_export.rs ~line 526
let (a, b, c, d, e, f) = (
    tex_matrix.x.x,  // scale X
    tex_matrix.x.y,  // shear Y  
    tex_matrix.y.x,  // shear X
    tex_matrix.y.y,  // scale Y
    tex_matrix.z.x,  // translate X
    tex_matrix.z.y,  // translate Y
);
```

---

## PDF Export Deep Dive

### Entry Point
```
generate_pdf() @ vector_export.rs:868
```

### Texture Embedding
```
embed_pdf_textures() @ vector_export.rs:820-865

For each texture with pixbuf:
1. Convert to RGB8
2. Compress with FlateDecode (zlib)
3. Create PDF Image XObject
4. Add to document resources
```

### Face Rendering Loop
```
generate_pdf_page_ops() @ vector_export.rs:992-1175

For each face with texture:
    // Draw base paper color
    ops.push("rg", [r, g, b])  // Set fill color
    ops.push("m", [x0, y0])    // Move to first vertex
    ops.push("l", [...])       // Lines to other vertices  
    ops.push("f", [])          // Fill path
    
    // Draw texture triangles
    triangles = triangulate_polygon(vertices.len())
    
    For each triangle:
        ops.push("q")              // Save graphics state
        
        // Clip to triangle
        ops.push("m", [x0, y0])
        ops.push("l", [x1, y1])
        ops.push("l", [x2, y2])
        ops.push("h")              // Close path
        ops.push("W")              // Set clipping path
        ops.push("n")              // End path (no stroke)
        
        // Transform and draw
        matrix = calc_pdf_texture_matrix_triangle(...)
        // Convert mm to points, flip Y
        ops.push("cm", [a, b, c, d, e, f])
        ops.push("Do", ["/Im{idx}"])
        
        ops.push("Q")              // Restore graphics state
```

### PDF Coordinate Conversion
```rust
// vector_export.rs ~line 1000
let mm_to_pt = |mm: f32| mm * 72.0 / 25.4;
let pdf_y = |y: f32| (page_size_mm.y - y) * 72.0 / 25.4;

// Matrix Y-flip for PDF
let a_pt = a * mm_to_pt_scale;
let b_pt = -b * mm_to_pt_scale;  // Negate!
let c_pt = c * mm_to_pt_scale;
let d_pt = -d * mm_to_pt_scale;  // Negate!
let e_pt = e * mm_to_pt_scale;
let f_pt = (page_size_mm.y - f) * mm_to_pt_scale;
```

---

## Matrix Mathematics

### Core Function
```
calc_texture_matrix() @ vector_export.rs:211-237
```

### The Problem
Map 3 UV points to 3 polygon vertices using affine transform:

```
M * [u]   [x]
    [v] = [y]
    [1]   [1]
```

### Solution
```rust
fn calc_texture_matrix(uvs: [Vector2; 3], pts: [Vector2; 3]) -> Option<Matrix3> {
    // Build UV matrix (columns = points in homogeneous coords)
    let u_mat = Matrix3::new(
        uvs[0].x, uvs[0].y, 1.0,  // Column 0
        uvs[1].x, uvs[1].y, 1.0,  // Column 1
        uvs[2].x, uvs[2].y, 1.0,  // Column 2
    );

    // Build target points matrix
    let p_mat = Matrix3::new(
        pts[0].x, pts[0].y, 1.0,
        pts[1].x, pts[1].y, 1.0,
        pts[2].x, pts[2].y, 1.0,
    );

    // M = P * U^(-1)
    u_mat.invert().map(|u_inv| p_mat * u_inv)
}
```

### UV to Pixel Conversion (SVG)
```rust
// calc_svg_texture_matrix_triangle() @ vector_export.rs:238-261
let pixel_uvs = [
    Vector2::new(uvs[0].x * tex_width, (1.0 - uvs[0].y) * tex_height),
    Vector2::new(uvs[1].x * tex_width, (1.0 - uvs[1].y) * tex_height),
    Vector2::new(uvs[2].x * tex_width, (1.0 - uvs[2].y) * tex_height),
];
```

The `1.0 - v` flip is critical for PDO format compatibility.

---

## Frontend Texture Handling

### Texture Loading
```typescript
// frontend/src/App.tsx:365-381
useEffect(() => {
    if (project?.model?.textures) {
        const loaded = project.model.textures.map((tex, i) => {
            if (!tex.has_data) return null;
            const img = new Image();
            img.src = `http://localhost:3000/api/texture/${i}`;
            img.onload = () => setRedrawKey(k => k + 1);
            return img;
        });
        setTextures(loaded);
    }
}, [project?.model?.textures]);
```

### Canvas 2D Rendering
```typescript
// frontend/src/App.tsx:469-515
const drawTexturedTriangle = (
    ctx: CanvasRenderingContext2D,
    img: HTMLImageElement,
    p0, p1, p2,  // Screen coordinates
    t0, t1, t2   // UV coordinates
) => {
    // Wrap UVs to [0,1]
    const wrapUV = (uv) => { ... };
    
    // Convert to pixel coords
    const u0 = wrapUV(t0.u) * w, v0 = wrapUV(t0.v) * h;
    // ...
    
    // Clip to triangle
    ctx.beginPath();
    ctx.moveTo(p0.x, p0.y);
    ctx.lineTo(p1.x, p1.y);
    ctx.lineTo(p2.x, p2.y);
    ctx.closePath();
    ctx.clip();
    
    // Calculate affine transform
    const det = (u1 - u0) * (v2 - v0) - (u2 - u0) * (v1 - v0);
    const a = ((v2 - v0) * (p1.x - p0.x) - (v1 - v0) * (p2.x - p0.x)) / det;
    // ... more matrix components
    
    ctx.transform(a, b, c, d, e, f);
    ctx.drawImage(img, 0, 0);
};
```

### Usage in Render Loop
```typescript
// frontend/src/App.tsx:1003-1041
if (showTexturesForFace) {
    for (let i = 1; i < indices.length - 1; i++) {  // Fan triangulation
        drawTexturedTriangle(
            ctx, texture,
            getPoint(face.vertices[0]),
            getPoint(face.vertices[i]),
            getPoint(face.vertices[i + 1]),
            { u: v0.t[0], v: 1.0 - v0.t[1] },  // V-flip!
            { u: v1.t[0], v: 1.0 - v1.t[1] },
            { u: v2.t[0], v: 1.0 - v2.t[1] }
        );
    }
}
```

Note: Frontend uses identical triangulation approach (fan from vertex 0).
