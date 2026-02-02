#[cfg(test)]
mod tests {
    use crate::util_3d::Vector2;
    use crate::vector_export::generate_svg_multipage;
    use cgmath::{Matrix3, SquareMatrix};
    use regex::Regex;
    use std::path::PathBuf;

    fn test_data_path(filename: &str) -> PathBuf {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("examples");
        d.push(filename);
        d
    }

    #[test]
    fn test_triangle_obj_svg_export() {
        let path = test_data_path("triangle.obj");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load triangle.obj");

        let svg = generate_svg_multipage(&papercraft, false).expect("Failed to generate SVG");

        // Verify SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("sodipodi:namedview"));
        assert!(svg.contains("inkscape:page"));
        assert!(svg.contains("Page_1"));

        // Check that faces are being drawn
        assert!(
            svg.contains("<polygon"),
            "SVG should contain polygon elements for faces"
        );
        assert!(svg.contains("fill=\"#"), "SVG should contain fill colors");
    }

    #[test]
    fn test_sphere_pdo_svg_export() {
        let path = test_data_path("sphere.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load sphere.pdo");

        let svg = generate_svg_multipage(&papercraft, false).expect("Failed to generate SVG");

        let re_group =
            Regex::new(r#"<g [^>]*id="page_(\d+)"[^>]*transform="translate\(([^,]+),([^)]+)\)""#)
                .unwrap();

        let mut found_pages = false;
        for cap in re_group.captures_iter(&svg) {
            found_pages = true;
            let page_num = &cap[1];
            let tx: f32 = cap[2].parse().unwrap();
            let ty: f32 = cap[3].parse().unwrap();
            println!("Page {}: Translate({}, {})", page_num, tx, ty);
        }
        assert!(found_pages, "Should have found page groups");
    }

    #[test]
    fn test_sphere_pdo_svg_export_with_textures() {
        let path = test_data_path("sphere.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load sphere.pdo");

        let svg = generate_svg_multipage(&papercraft, true).expect("Failed to generate SVG");

        // This confirms that we are generating texture logic
        // Updated to expect <pattern> elements for texture wrapping support
        let has_patterns = svg.contains("<pattern");
        let has_pattern_fills = svg.contains("fill=\"url(#pat_");

        let has_textures_with_data = papercraft.model().textures().any(|t| t.pixbuf().is_some());
        if has_textures_with_data {
            assert!(has_patterns, "Should have pattern definitions for textures");
            assert!(has_pattern_fills, "Should have pattern fills on polygons");
        }
    }

    #[test]
    fn test_texture_matrix_calculation_logic() {
        // Verify the matrix calculation logic is correct (without transpose)

        let uvs = [
            Vector2::new(0.25, 0.25),
            Vector2::new(0.75, 0.25),
            Vector2::new(0.5, 0.75),
        ];

        let vertices = [
            Vector2::new(50.0, 100.0),
            Vector2::new(80.0, 100.0),
            Vector2::new(65.0, 130.0),
        ];

        // Build matrices without transpose
        let u_mat = Matrix3::new(
            uvs[0].x, uvs[0].y, 1.0, uvs[1].x, uvs[1].y, 1.0, uvs[2].x, uvs[2].y, 1.0,
        );

        let p_mat = Matrix3::new(
            vertices[0].x,
            vertices[0].y,
            1.0,
            vertices[1].x,
            vertices[1].y,
            1.0,
            vertices[2].x,
            vertices[2].y,
            1.0,
        );

        let u_inv = u_mat.invert().expect("Should be invertible");
        let matrix = p_mat * u_inv;

        // Verify mapping
        for i in 0..3 {
            let uv = cgmath::Vector3::new(uvs[i].x, uvs[i].y, 1.0);
            let result = matrix * uv;

            assert!((result.x - vertices[i].x).abs() < 0.01);
            assert!((result.y - vertices[i].y).abs() < 0.01);
        }
    }

    // ==========================================================================
    // DICE.PDO TEXTURE TESTS - These tests verify actual texture rendering
    // ==========================================================================

    /// Simple test: Export dice.pdo to SVG and verify it contains raster image data.
    /// This is the most basic test for texture export - if this fails, textures are broken.
    #[test]
    fn test_dice_svg_contains_raster_images() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        // Export to SVG with textures enabled
        let svg = generate_svg_multipage(&papercraft, true).expect("Failed to generate SVG");

        // The SVG must contain <image> elements with embedded raster data
        assert!(
            svg.contains("<image"),
            "SVG export of dice.pdo must contain <image> elements for textures"
        );

        // The SVG must contain base64-encoded PNG data (the actual raster bitmap)
        assert!(
            svg.contains("data:image/png;base64,"),
            "SVG export of dice.pdo must contain base64-encoded PNG raster data"
        );

        // Verify the image data is substantial (not empty/placeholder)
        // A real PNG base64 string is at least a few hundred characters
        let has_real_image_data = svg
            .split("data:image/png;base64,")
            .skip(1) // Skip the part before first match
            .any(|part| {
                // Get the base64 data up to the closing quote
                let base64_data = part.split('"').next().unwrap_or("");
                base64_data.len() > 200
            });

        assert!(
            has_real_image_data,
            "SVG must contain actual image data (base64 length > 200 chars)"
        );
    }

    /// Test that dice.pdo loads with texture data extracted from the PDO file.
    /// dice.pdo is a simple 6-sided die with 6 different textures (one per face).
    #[test]
    fn test_dice_pdo_textures_are_extracted() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        // dice.pdo has 6 textures (one per face of the die)
        let textures: Vec<_> = papercraft.model().textures().collect();

        // Verify we have textures
        assert!(
            !textures.is_empty(),
            "dice.pdo should have textures defined"
        );

        // Count textures with actual pixel data
        let textures_with_data: Vec<_> = textures.iter().filter(|t| t.pixbuf().is_some()).collect();

        // CRITICAL: This tests that texture pixel data was actually extracted
        assert!(
            !textures_with_data.is_empty(),
            "dice.pdo textures should have pixel data (pixbuf). \
             Found {} textures but none have pixel data. \
             Texture names: {:?}",
            textures.len(),
            textures.iter().map(|t| t.file_name()).collect::<Vec<_>>()
        );

        // Verify texture dimensions are reasonable (not 0x0)
        for (i, tex) in textures_with_data.iter().enumerate() {
            let pixbuf = tex.pixbuf().unwrap();
            assert!(
                pixbuf.width() > 0 && pixbuf.height() > 0,
                "Texture {} should have valid dimensions, got {}x{}",
                i,
                pixbuf.width(),
                pixbuf.height()
            );
        }
    }

    /// Test that SVG export with textures actually contains embedded image data.
    /// This catches the bug where SVG shows blank colors instead of textures.
    #[test]
    fn test_dice_pdo_svg_export_contains_embedded_textures() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        let svg = generate_svg_multipage(&papercraft, true).expect("Failed to generate SVG");

        // Check for texture image definitions in <defs>
        let has_tex_image = svg.contains("<image id=\"tex_");
        assert!(
            has_tex_image,
            "SVG should contain <image id=\"tex_...> elements in defs"
        );

        // Check for base64 encoded PNG data
        let has_base64_png = svg.contains("data:image/png;base64,");
        assert!(
            has_base64_png,
            "SVG should contain base64-encoded PNG texture data"
        );

        // Verify the base64 data is not empty (more than just the prefix)
        let re_image =
            Regex::new(r#"<image id="tex_\d+" [^>]*href="data:image/png;base64,([^"]+)""#).unwrap();
        let mut found_valid_texture = false;
        for cap in re_image.captures_iter(&svg) {
            let base64_data = &cap[1];
            // A valid PNG base64 should be at least 100 chars
            if base64_data.len() > 100 {
                found_valid_texture = true;
                break;
            }
        }
        assert!(
            found_valid_texture,
            "SVG should contain at least one texture with substantial base64 data"
        );
    }

    /// Test that SVG export creates proper patterns and transforms for textured faces.
    #[test]
    fn test_dice_pdo_svg_export_has_texture_transforms() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        let svg = generate_svg_multipage(&papercraft, true).expect("Failed to generate SVG");

        // Check for patterns (used to wrap/tile textures)
        let pattern_count = svg.matches("<pattern").count();
        assert!(
            pattern_count > 0,
            "SVG should have patterns for texture triangles, found 0"
        );

        // Check for pattern transforms
        let transform_count = svg.matches("patternTransform=\"matrix(").count();
        assert!(
            transform_count > 0,
            "SVG should have matrix transforms on patterns, found 0"
        );

        // The number of patterns and transforms should be equal
        assert_eq!(
            pattern_count, transform_count,
            "Pattern count ({}) and transform count ({}) should be equal",
            pattern_count, transform_count
        );
    }

    /// Test that UV coordinates are correctly extracted from dice.pdo faces.
    #[test]
    fn test_dice_pdo_uv_coordinates_are_valid() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        let model = papercraft.model();

        // Check that vertices have valid UV coordinates
        let mut total_vertices = 0;
        let mut vertices_with_valid_uvs = 0;

        for (_, face) in model.faces() {
            for vertex_idx in face.index_vertices() {
                total_vertices += 1;
                let vertex = &model[vertex_idx];
                let uv = vertex.uv();

                // UVs should typically be in 0-1 range (or slightly outside for tiling)
                // But they should not be exactly (0,0) for all vertices
                if uv.x != 0.0 || uv.y != 0.0 {
                    vertices_with_valid_uvs += 1;
                }

                // UVs should be finite numbers
                assert!(
                    uv.x.is_finite() && uv.y.is_finite(),
                    "UV coordinates should be finite numbers, got ({}, {})",
                    uv.x,
                    uv.y
                );
            }
        }

        // At least some vertices should have non-zero UVs
        assert!(
            vertices_with_valid_uvs > 0,
            "At least some vertices should have non-zero UV coordinates. \
             Found {} total vertices but {} with valid UVs",
            total_vertices,
            vertices_with_valid_uvs
        );

        // Most vertices should have valid UVs (allow some at origin for models with partial texturing)
        let ratio = (vertices_with_valid_uvs as f32) / (total_vertices as f32);
        assert!(
            ratio > 0.5,
            "At least 50% of vertices should have non-zero UVs, got {:.1}%",
            ratio * 100.0
        );
    }

    /// Test that PDF export with textures contains XObject image streams.
    #[test]
    fn test_dice_pdo_pdf_export_contains_textures() {
        use crate::vector_export::generate_pdf;

        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        let pdf_bytes = generate_pdf(&papercraft, true).expect("Failed to generate PDF");

        // PDF should have reasonable size (with textures embedded)
        assert!(
            pdf_bytes.len() > 1000,
            "PDF should be larger than 1KB when textures are embedded"
        );

        // Check PDF magic bytes
        assert!(pdf_bytes.starts_with(b"%PDF"), "Should be a valid PDF file");

        // Convert to string for text-based checks (PDF is partially text)
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);

        // Check for XObject image references
        let has_xobject = pdf_str.contains("/XObject");
        assert!(
            has_xobject,
            "PDF should contain /XObject dictionary for textures"
        );

        // Check for Image XObject type
        let has_image_xobject =
            pdf_str.contains("/Subtype /Image") || pdf_str.contains("/Subtype/Image");
        assert!(
            has_image_xobject,
            "PDF should contain Image XObject definitions"
        );

        // Check for image reference in pattern content stream (Do operator)
        // Since we now use Patterns, the Do operator appears inside the Pattern stream
        let has_do_operator = pdf_str.contains("/Im Do") || pdf_str.contains("/Im0 Do");
        assert!(
            has_do_operator,
            "PDF should reference textures with Do operator (inside Pattern)"
        );

        // Check for Pattern usage in page content
        let has_pattern_cs = pdf_str.contains("/Pattern cs");
        assert!(
            has_pattern_cs,
            "PDF page content should use Pattern color space"
        );

        let has_pattern_scn = pdf_str.contains("/Pat0 scn") || pdf_str.contains("/Pat1 scn");
        assert!(
            has_pattern_scn,
            "PDF page content should set pattern color (scn)"
        );
    }

    /// Test that exporting without textures flag produces solid color fill (not textures).
    #[test]
    fn test_dice_pdo_svg_export_without_textures_shows_solid_colors() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        // Export WITHOUT textures
        let svg = generate_svg_multipage(&papercraft, false).expect("Failed to generate SVG");

        // Should have polygons with solid fill colors
        assert!(
            svg.contains("<polygon"),
            "SVG should contain polygon elements"
        );
        assert!(
            svg.contains("fill=\"#"),
            "SVG should contain solid color fills"
        );

        // Should NOT have texture-related elements
        assert!(
            !svg.contains("<image id=\"tex_"),
            "SVG without textures should not contain texture image definitions"
        );
        assert!(
            !svg.contains("data:image/png;base64,"),
            "SVG without textures should not contain base64 image data"
        );
    }

    /// Comprehensive integration test: Export dice.pdo to SVG and verify it renders textures.
    /// This is the main test that should FAIL if textures are showing as blank.
    #[test]
    fn test_dice_pdo_svg_texture_rendering_integration() {
        let path = test_data_path("dice.pdo");
        let (papercraft, _) =
            crate::paper::import::import_model_file(&path).expect("Failed to load dice.pdo");

        // First, verify model has textures with data
        let textures_with_data: Vec<_> = papercraft
            .model()
            .textures()
            .filter(|t| t.pixbuf().is_some())
            .collect();

        assert!(
            !textures_with_data.is_empty(),
            "PRECONDITION: dice.pdo must have textures with pixel data for this test"
        );

        // Generate SVG with textures
        let svg = generate_svg_multipage(&papercraft, true).expect("Failed to generate SVG");

        // CRITICAL CHECK 1: Embedded texture images exist in defs
        let re_tex_def =
            Regex::new(r#"<image id="tex_(\d+)"[^>]*width="(\d+)"[^>]*height="(\d+)""#).unwrap();
        let tex_defs: Vec<_> = re_tex_def.captures_iter(&svg).collect();
        assert!(
            !tex_defs.is_empty(),
            "FAILURE: No texture definitions found in SVG defs section. \
             Textures are not being embedded."
        );

        // CRITICAL CHECK 2: Texture dimensions match loaded textures
        for cap in &tex_defs {
            let tex_idx: usize = cap[1].parse().unwrap();
            let svg_width: u32 = cap[2].parse().unwrap();
            let svg_height: u32 = cap[3].parse().unwrap();

            if let Some(tex) = papercraft.model().textures().nth(tex_idx) {
                if let Some(pixbuf) = tex.pixbuf() {
                    assert_eq!(
                        svg_width,
                        pixbuf.width(),
                        "Texture {} width mismatch",
                        tex_idx
                    );
                    assert_eq!(
                        svg_height,
                        pixbuf.height(),
                        "Texture {} height mismatch",
                        tex_idx
                    );
                }
            }
        }

        // CRITICAL CHECK 3: Pattern definitions exist with userSpaceOnUse
        // We now use patterns instead of clip paths to support texture wrapping
        let re_pattern =
            Regex::new(r#"<pattern id="pat_face_\d+_\d+" patternUnits="userSpaceOnUse""#).unwrap();
        let patterns: Vec<_> = re_pattern.captures_iter(&svg).collect();
        assert!(
            !patterns.is_empty(),
            "FAILURE: No texture patterns found. Texture rendering should use patterns."
        );

        // CRITICAL CHECK 4: Pattern transforms with valid matrix values
        let re_transform = Regex::new(r#"patternTransform="matrix\(([^)]+)\)""#).unwrap();
        let transforms: Vec<_> = re_transform.captures_iter(&svg).collect();
        assert!(
            !transforms.is_empty(),
            "FAILURE: No pattern transforms found."
        );

        // Verify transform matrices have valid values (not all zeros or NaN)
        for cap in &transforms {
            let matrix_str = &cap[1];
            let values: Vec<f32> = matrix_str
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            assert_eq!(
                values.len(),
                6,
                "Matrix transform should have 6 values (a b c d e f)"
            );

            // Check all values are finite
            for (i, val) in values.iter().enumerate() {
                assert!(
                    val.is_finite(),
                    "Matrix value {} should be finite, got {}",
                    i,
                    val
                );
            }

            // The matrix should represent a valid affine transform (non-degenerate)
            // We check that the linear part (2x2 submatrix) is not all zeros.
            // Note: A 90-degree rotation will have a=0, d=0 but b!=0, c!=0.
            let (a, b, c, d) = (values[0], values[1], values[2], values[3]);
            let has_scale_or_rotation =
                a.abs() > 0.0001 || b.abs() > 0.0001 || c.abs() > 0.0001 || d.abs() > 0.0001;

            assert!(
                has_scale_or_rotation,
                "Matrix linear components (a,b,c,d) should not all be zero. Got: a={}, b={}, c={}, d={}",
                a, b, c, d
            );
        }

        // CRITICAL CHECK 5: Polygons use the patterns for filling
        let re_fill = Regex::new(r#"fill="url\(#pat_face_\d+_\d+\)""#).unwrap();
        let fills: Vec<_> = re_fill.captures_iter(&svg).collect();
        assert!(
            !fills.is_empty(),
            "FAILURE: No polygons found filled with texture patterns."
        );

        println!(
            "SVG texture rendering test passed: {} texture defs, {} patterns, {} fills",
            tex_defs.len(),
            patterns.len(),
            fills.len()
        );
    }
}
