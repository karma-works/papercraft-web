
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::paper::Papercraft;
    use crate::vector_export::generate_svg_multipage;
    use regex::Regex;

    fn test_data_path(filename: &str) -> PathBuf {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("examples");
        d.push(filename);
        d
    }

    #[test]
    fn test_triangle_obj_svg_export() {
        let path = test_data_path("triangle.obj");
        let (papercraft, _) = crate::paper::import::import_model_file(&path).expect("Failed to load triangle.obj");
        
        let svg = generate_svg_multipage(&papercraft).expect("Failed to generate SVG");
        
        // Verify SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("sodipodi:namedview"));
        assert!(svg.contains("inkscape:page"));
        assert!(svg.contains("Page_1"));
    }

    #[test]
    fn test_sphere_pdo_svg_export() {
        let path = test_data_path("sphere.pdo");
        // Using explicit crate::paper::import path
        let (papercraft, _) = crate::paper::import::import_model_file(&path).expect("Failed to load sphere.pdo");
        
        let svg = generate_svg_multipage(&papercraft).expect("Failed to generate SVG");
        
        // Regex to find groups and transforms
        let re_group = Regex::new(r#"<g [^>]*id="page_(\d+)"[^>]*transform="translate\(([^,]+),([^)]+)\)""#).unwrap();
        
        let mut found_pages = false;
        for cap in re_group.captures_iter(&svg) {
            found_pages = true;
            let page_num = &cap[1];
            let tx: f32 = cap[2].parse().unwrap();
            let ty: f32 = cap[3].parse().unwrap();
            
            println!("Page {}: Translate({}, {})", page_num, tx, ty);
            
            // For sphere.pdo (likely multi-page), tx/ty should reflect grid layout.
            // If it's pure vertical or horizontal, we can check.
        }
        assert!(found_pages, "Should have found page groups");
        
        // Check for coordinates that are wildly off-page RELATIVE to the group.
        // SVG polygon points: points="x1,y1 x2,y2 ..."
        // We expect x,y to be within [0, page_width] and [0, page_height] roughly.
        // A4 is 210x297mm.
        // Let's allow some margin/padding, so -10 to 310 maybe.
        
        let re_poly = Regex::new(r#"points="([^"]+)""#).unwrap();
        for cap in re_poly.captures_iter(&svg) {
            let points_str = &cap[1];
            for pair in points_str.trim().split(' ') {
                if pair.is_empty() { continue; }
                let coords: Vec<&str> = pair.split(',').collect();
                if coords.len() == 2 {
                    let x: f32 = coords[0].parse().unwrap();
                    let y: f32 = coords[1].parse().unwrap();
                    
                    // Assertion: Elements should be roughly within A4 bounds
                    assert!(x > -50.0 && x < 350.0, "Point X {} out of bounds", x);
                    assert!(y > -50.0 && y < 400.0, "Point Y {} out of bounds", y);
                }
            }
        }
    }
}
