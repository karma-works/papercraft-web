
---

## SVG Texturing Fundamentals

SVG (Scalable Vector Graphics) does not have native "texture" support like WebGL shaders. Instead, textures are applied through coordinate mapping techniques using affine transforms and clipping. Key concepts:

### 1. Affine Transform Limitations
- **Affine transforms** can only correctly map **3 points** (a triangle)
- For polygons with 4+ vertices, a single transform will distort mapping for vertices 4+
- **Solution**: Triangulate faces into triangles, apply separate transforms/clips per triangle

### 2. Texture Embedding in SVG
- Images are embedded in `<defs>` section with base64-encoded PNG data:
  ```svg
  <defs>
    <image id="tex_0" width="512" height="512" preserveAspectRatio="none" href="data:image/png;base64,..."/>
  </defs>
  ```
- `<use>` elements reference textures with transforms: `<use href="#tex_0" transform="matrix(...)"/>`
- `<clipPath>` elements restrict drawing to specific polygons

### 3. Coordinate Space Challenges
- UV coordinates: (0,0) to (1,1) normalized texture space
- Pixel coordinates: UVs scaled by texture dimensions (UV * width/height)
- Y-axis flip: SVG images are top-down; PDO format is bottom-up, requiring V inversion
- Homogeneous coordinates: 3x3 matrices for UV-to-world mapping

### 4. Triangulation Approach
- Complex faces split into triangles using fan triangulation
- Each triangle gets its own clip path and transform matrix
- Works for convex polygons (typ. faces in papercraft models)

### 5. Limitations vs WebGL
- **No GPU processing**: No per-pixel UV interpolation
- **No perspective-correct mapping**: Affine-only (linear) transforms
- **No filtering**: Relies on browser rasterization
- **Performance**: Large textures increase file size/render time

---

## PDO Format: Pepakura Designer Model Data

### 1. Format Overview
- **PDO (Pepakura Designer Version)**: Proprietary 3D model format by Tamiya
- **Binary format** optimized for unfoldable paper models
- Includes embedded textures, UV coordinates, and fold/join metadata

### 2. Data Structure
```
PDO File Structure:
├── Header (magic bytes, version, model info)
├── Vertices (X,Y,Z positions)
├── UV Coordinates (per-vertex texture mapping)
├── Faces (vertex indices, material indices)
├── Materials (textures, colors)
├── Folding Data (edge types: cut/fold/join)
└── Island Data (pre-defined paper piece groupings)
```

### 3. Texture Storage
- **Embedded textures**: PNG/JPG data stored directly in PDO file
- **UV coordinate space**: V increases downward (opposite of OpenGL/SVG convention)
- **Material indices**: Zero-based texture references from faces

### 4. Edge Classifications
- **Cut edges**: Full separation lines (ink+scissors)
- **Fold edges**: Dashed/dotted lines with angle direction
- **Join edges**: Glue flap connections with thickness

### 5. Import Process
- Custom importer in `backend/src/paper/model/import/pepakura/`
- Parses binary structure to extract geometry/textures
- Converts to unified `Papercraft` model representation
- Handles multiple texture layers and material assignments
