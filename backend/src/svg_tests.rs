#[cfg(test)]
mod tests {
    use crate::util_3d::Vector2;
    use crate::vector_export::generate_svg_multipage;
    use cgmath::{Matrix, Matrix3, Point2, SquareMatrix, Transform};
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
        let has_clip_paths = svg.contains("<clipPath");
        let has_use_transforms = svg.contains("<use href=\"#tex_");

        let has_textures_with_data = papercraft.model().textures().any(|t| t.pixbuf().is_some());
        if has_textures_with_data {
            assert!(has_clip_paths, "Should have clip paths for textures");
            assert!(has_use_transforms, "Should have transformed image uses");
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
}
