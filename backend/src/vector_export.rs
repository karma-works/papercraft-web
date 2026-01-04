//! Pure vector PDF/SVG export module.
//!
//! Generates vector output without OpenGL or imgui dependencies.

use anyhow::Result;
use std::io::Write;
use std::ops::ControlFlow;
use cgmath::{EuclideanSpace, InnerSpace, Rad, Transform};

use crate::paper::{
    EdgeIdPosition, EdgeStatus, FlapStyle, FoldStyle, IslandKey, Papercraft,
    signature,
};
use crate::util_3d::{Matrix3, Point2, Vector2};
use cgmath::SquareMatrix;

/// Text alignment for labels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Near,
    Center,
    Far,
}

/// A text element to render
#[derive(Debug, Clone)]
pub struct PrintableText {
    pub size: f32,
    pub pos: Vector2,
    pub angle: Rad<f32>,
    pub align: TextAlign,
    pub text: String,
}

/// Edge classification for fold lines
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum EdgeDrawKind {
    Mountain,
    Valley,
}

// Font size for page footer
const FONT_SIZE: f32 = 3.0;

/// Generate a single-page SVG for the given papercraft project.
///
/// Returns the SVG as a string.
pub fn generate_svg(papercraft: &Papercraft, page: u32) -> Result<String> {
    let mut output = Vec::new();
    write_svg_page(papercraft, page, &mut output)?;
    Ok(String::from_utf8(output)?)
}

/// Generate a multi-page SVG (Inkscape-style with sodipodi:namedview).
pub fn generate_svg_multipage(papercraft: &Papercraft) -> Result<String> {
    let mut output = Vec::new();
    write_svg_multipage(papercraft, &mut output)?;
    Ok(String::from_utf8(output)?)
}

/// Write a single SVG page to the given writer.
fn write_svg_page(papercraft: &Papercraft, page: u32, w: &mut impl Write) -> Result<()> {
    let options = papercraft.options();
    let page_size = Vector2::new(options.page_size.0, options.page_size.1);

    // SVG Header
    writeln!(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#)?;
    writeln!(
        w,
        r#"<svg width="{0}mm" height="{1}mm" viewBox="0 0 {0} {1}" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape">"#,
        page_size.x, page_size.y
    )?;

    // Write all layers
    write_svg_layers(papercraft, page, w)?;

    writeln!(w, r#"</svg>"#)?;
    Ok(())
}

/// Write multi-page SVG.
fn write_svg_multipage(papercraft: &Papercraft, w: &mut impl Write) -> Result<()> {
    let options = papercraft.options();
    let page_size = Vector2::new(options.page_size.0, options.page_size.1);
    let page_count = options.pages;

    // Calculate total canvas size
    // We must match backend/src/paper/craft.rs PAGE_SEP
    const PAGE_SEP: f32 = 10.0; 
    let cols = options.page_cols.max(1);
    let rows = (page_count + cols - 1) / cols;
    
    let total_width = (cols as f32) * (page_size.x + PAGE_SEP) - PAGE_SEP;
    let total_height = (rows as f32) * (page_size.y + PAGE_SEP) - PAGE_SEP;

    // SVG Header
    writeln!(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#)?;
    writeln!(
        w,
        r#"<svg width="{0}mm" height="{1}mm" viewBox="0 0 {0} {1}" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd">"#,
        total_width, total_height
    )?;

    // Inkscape page definitions
    writeln!(w, r#"<sodipodi:namedview>"#)?;
    for p in 0..page_count {
        let page_offset = options.page_position(p);
        writeln!(
            w,
            r#"<inkscape:page x="{}" y="{}" width="{}" height="{}" id="Page_{}" />"#,
            page_offset.x, page_offset.y, page_size.x, page_size.y, p + 1
        )?;
    }
    writeln!(w, r#"</sodipodi:namedview>"#)?;

    // Write each page as a group
    for p in 0..page_count {
        let page_offset = options.page_position(p);
        writeln!(
            w,
            r#"<g inkscape:label="Page_{}" inkscape:groupmode="layer" id="page_{}" transform="translate({},{})">"#,
            p + 1, p + 1, page_offset.x, page_offset.y
        )?;
        write_svg_layers(papercraft, p, w)?;
        writeln!(w, r#"</g>"#)?;
    }

    writeln!(w, r#"</svg>"#)?;
    Ok(())
}

/// Write all SVG layers for a single page.
fn write_svg_layers(papercraft: &Papercraft, page: u32, w: &mut impl Write) -> Result<()> {
    let options = papercraft.options();
    let scale = options.scale;

    // Collect data
    let mut faces_data: Vec<(IslandKey, Vec<Vector2>)> = Vec::new();
    let mut cut_paths: Vec<Vec<Vector2>> = Vec::new();
    let mut mountain_lines: Vec<(Vector2, Vector2)> = Vec::new();
    let mut valley_lines: Vec<(Vector2, Vector2)> = Vec::new();
    let mut flap_polygons: Vec<Vec<Vector2>> = Vec::new();

    // Page slot geometry for relaxed assignment
    let _slot_width = match options.page_cols {
        0 | 1 => 100000.0,
        _ => options.page_size.0 + 10.0, // Width + Sep
    };
    let _slot_height = options.page_size.1 + 10.0;
    let _page_cols = options.page_cols.max(1);

    // Iterate over all islands
    for (i_island, island) in papercraft.islands() {
        // Determine which page this island belongs to based on bounding box center
        let (bb_min, bb_max) = papercraft.island_bounding_box_angle(island, Rad(0.0));
        let center = (bb_min + bb_max) / 2.0;

        let po = options.global_to_page(center);
        let owner_page = (po.row as u32) * options.page_cols.max(1) + (po.col as u32);
        
        // If this island does not belong to the current page, skip it completely.
        if owner_page != page {
            continue;
        }

        let page_offset = options.page_position(page);

        // 1. Build Face -> Island matrix map
        let mut face_matrices: std::collections::HashMap<crate::paper::FaceIndex, Matrix3> = std::collections::HashMap::new();
        let _ = papercraft.traverse_faces(island, |i_face, _, mx| {
            face_matrices.insert(i_face, *mx);
            ControlFlow::Continue(())
        });

        // 2. Collect Faces
        let _ = papercraft.traverse_faces(island, |_, face, full_mx| {
            let plane = papercraft.model().face_plane(face);
            
            let mut face_vertices: Vec<Vector2> = Vec::new();
            for i_vertex in face.index_vertices() {
                let vertex = &papercraft.model()[i_vertex];
                let v2d = plane.project(&vertex.pos(), scale);
                let transformed = full_mx.transform_point(Point2::from_vec(v2d)).to_vec();
                
                // Always convert to relative, no filtering
                face_vertices.push(transformed - page_offset);
            }
            faces_data.push((i_island, face_vertices));
            ControlFlow::Continue(())
        });

        // 3. Collect Cut Paths (Perimeter)
        let perimeter = papercraft.island_perimeter(i_island);
        if !perimeter.is_empty() {
            let mut contour_points: Vec<Vector2> = Vec::new();
            
            for peri in perimeter.iter() {
                let edge = &papercraft.model()[peri.i_edge()];
                let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
                let face = &papercraft.model()[i_face];
                let plane = papercraft.model().face_plane(face);
                
                let full_mx = face_matrices.get(&i_face).cloned().unwrap_or(Matrix3::identity());

                let (i_v0, _) = face.vertices_of_edge(peri.i_edge()).unwrap();
                let v0 = &papercraft.model()[i_v0];
                
                let p0_2d = plane.project(&v0.pos(), scale);
                let p0 = full_mx.transform_point(Point2::from_vec(p0_2d)).to_vec();
                
                contour_points.push(p0 - page_offset);
            }
            
            if !contour_points.is_empty() {
                cut_paths.push(contour_points);
            }
        }

        // 4. Collect Folds
        let _ = papercraft.traverse_faces(island, |i_face, face, full_mx| {
             for i_edge in face.index_edges() {
                let edge_status = papercraft.edge_status(i_edge);
                if edge_status != EdgeStatus::Joined { continue; }

                let edge = &papercraft.model()[i_edge];
                let (_f_a, f_b_opt) = edge.faces();
                let Some(f_b) = f_b_opt else { continue; };
                
                if i_face > f_b { continue; }
                if i_face == f_b { continue; }

                let plane = papercraft.model().face_plane(face);
                let Some((i_v0, i_v1)) = face.vertices_of_edge(i_edge) else { continue; };
                
                let v0 = &papercraft.model()[i_v0];
                let v1 = &papercraft.model()[i_v1];
                
                let p0 = full_mx.transform_point(Point2::from_vec(plane.project(&v0.pos(), scale))).to_vec();
                let p1 = full_mx.transform_point(Point2::from_vec(plane.project(&v1.pos(), scale))).to_vec();
                
                let p0_rel = p0 - page_offset;
                let p1_rel = p1 - page_offset;
                
                let angle = edge.angle().0;
                if angle.is_sign_negative() {
                    valley_lines.push((p0_rel, p1_rel));
                } else {
                    mountain_lines.push((p0_rel, p1_rel));
                }
             }
             ControlFlow::Continue(())
        });

        // 5. Collect Flaps
        if options.flap_style != FlapStyle::None {
            for peri in papercraft.island_perimeter(i_island).iter() {
                let edge_status = papercraft.edge_status(peri.i_edge());
                if let EdgeStatus::Cut(flap_side) = edge_status {
                    if !flap_side.flap_visible(peri.face_sign()) { continue; }

                    let edge = &papercraft.model()[peri.i_edge()];
                    let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
                    let face = &papercraft.model()[i_face];
                    let plane = papercraft.model().face_plane(face);
                    
                    let full_mx = face_matrices.get(&i_face).cloned().unwrap_or(Matrix3::identity());

                    let Some((i_v0, i_v1)) = face.vertices_of_edge(peri.i_edge()) else { continue; };
                    let v0 = &papercraft.model()[i_v0];
                    let v1 = &papercraft.model()[i_v1];
                    
                    let p0 = full_mx.transform_point(Point2::from_vec(plane.project(&v0.pos(), scale))).to_vec();
                    let p1 = full_mx.transform_point(Point2::from_vec(plane.project(&v1.pos(), scale))).to_vec();

                    let p0_rel = p0 - page_offset;
                    let p1_rel = p1 - page_offset;

                    // Calculation of flap geometry needs original vector direction, 
                    // but we can use relative points since vector difference is same.
                    let edge_vec = p1_rel - p0_rel;
                    let edge_len = edge_vec.magnitude();
                    let normal = Vector2::new(-edge_vec.y, edge_vec.x).normalize();
                    let flap_width = options.flap_width.min(edge_len * 0.4);
                    let taper = 0.15;
                    
                    let f0 = p0_rel + normal * flap_width + edge_vec.normalize() * (edge_len * taper);
                    let f1 = p1_rel + normal * flap_width - edge_vec.normalize() * (edge_len * taper);
                    
                    flap_polygons.push(vec![p0_rel, p1_rel, f1, f0]);
                }
            }
        }
    }

    // Colors from options
    let paper_color_hex = options.paper_color.to_hex();
    let cut_color_hex = options.cut_line_color.to_hex();
    let fold_color_hex = options.fold_line_color.to_hex();
    let tab_color_hex = options.tab_line_color.to_hex();

    writeln!(w, r#"<g inkscape:label="Faces" inkscape:groupmode="layer" id="Faces">"#)?;
    for (idx, (_, vertices)) in faces_data.iter().enumerate() {
        if vertices.len() >= 3 {
            write!(w, r#"<polygon id="face_{}" fill="{}" stroke="none" points=""#, idx, paper_color_hex)?;
            for v in vertices { write!(w, "{},{} ", v.x, v.y)?; }
            writeln!(w, r#""/>"#)?;
        }
    }
    writeln!(w, r#"</g>"#)?;

    // Write Flaps layer
    if !flap_polygons.is_empty() {
        writeln!(w, r#"<g inkscape:label="Flaps" inkscape:groupmode="layer" id="Flaps">"#)?;
        for (idx, vertices) in flap_polygons.iter().enumerate() {
            write!(w, r##"<polygon id="flap_{}" fill="#E0E0E0" stroke="{}" stroke-width="0.1" points=""##, idx, tab_color_hex)?;
            for v in vertices { write!(w, "{},{} ", v.x, v.y)?; }
            writeln!(w, r#""/>"#)?;
        }
        writeln!(w, r#"</g>"#)?;
    }

    // Write Fold lines layer
    if options.fold_style != FoldStyle::None {
        writeln!(w, r#"<g inkscape:label="Fold" inkscape:groupmode="layer" id="Fold">"#)?;
        writeln!(w, r#"<g inkscape:label="Mountain" inkscape:groupmode="layer" id="Mountain">"#)?;
        for (idx, (p0, p1)) in mountain_lines.iter().enumerate() {
            writeln!(w, r##"<line id="mountain_{}" x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.2" />"##, idx, p0.x, p0.y, p1.x, p1.y, fold_color_hex)?;
        }
        writeln!(w, r#"</g>"#)?;
        writeln!(w, r#"<g inkscape:label="Valley" inkscape:groupmode="layer" id="Valley">"#)?;
        for (idx, (p0, p1)) in valley_lines.iter().enumerate() {
            writeln!(w, r##"<line id="valley_{}" x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.2" stroke-dasharray="1,1"/>"##, idx, p0.x, p0.y, p1.x, p1.y, fold_color_hex)?;
        }
        writeln!(w, r#"</g></g>"#)?;
    }

    // Write Cut lines layer
    writeln!(w, r#"<g inkscape:label="Cut" inkscape:groupmode="layer" id="Cut">"#)?;
    for (idx, contour) in cut_paths.iter().enumerate() {
        write!(w, r##"<path id="cut_{}" fill="none" stroke="{}" stroke-width="0.3" d="M "##, idx, cut_color_hex)?;
        for (i, p) in contour.iter().enumerate() {
            if i == 0 { write!(w, "{},{} ", p.x, p.y)?; } 
            else { write!(w, "L {},{} ", p.x, p.y)?; }
        }
        writeln!(w, r#"Z"/>"#)?;
    }
    writeln!(w, r#"</g>"#)?;

    // Write Text layer
    let texts = collect_texts(papercraft, page);
    if !texts.is_empty() {
        writeln!(w, r#"<g inkscape:label="Text" inkscape:groupmode="layer" id="Text">"#)?;
        for text in texts {
            let anchor = match text.align {
                TextAlign::Near => "",
                TextAlign::Center => "text-anchor:middle;",
                TextAlign::Far => "text-anchor:end;",
            };
            let angle_deg = text.angle.0.to_degrees();
            if angle_deg.abs() < 0.01 {
                writeln!(w, r#"<text x="{}" y="{}" style="{}font-size:{}px;font-family:sans-serif;fill:#000000">{}</text>"#, text.pos.x, text.pos.y, anchor, text.size, html_escape(&text.text))?;
            } else {
                writeln!(w, r#"<text x="{}" y="{}" style="{}font-size:{}px;font-family:sans-serif;fill:#000000" transform="rotate({} {} {})">{}</text>"#, text.pos.x, text.pos.y, anchor, text.size, angle_deg, text.pos.x, text.pos.y, html_escape(&text.text))?;
            }
        }
        writeln!(w, r#"</g>"#)?;
    }

    Ok(())
}

/// Collect text elements for a page (page numbers, edge IDs, signature).
fn collect_texts(papercraft: &Papercraft, page: u32) -> Vec<PrintableText> {
    let options = papercraft.options();
    let page_size = Vector2::new(options.page_size.0, options.page_size.1);
    let (_margin_top, margin_left, margin_right, margin_bottom) = options.margin;
    let page_count = options.pages;
    
    let mut texts = Vec::new();

    // Signature
    if options.show_self_promotion {
        let x = margin_left;
        let y = (page_size.y - margin_bottom + FONT_SIZE).min(page_size.y - FONT_SIZE);
        texts.push(PrintableText {
            size: FONT_SIZE,
            pos: Vector2::new(x, y),
            angle: Rad(0.0),
            align: TextAlign::Near,
            text: signature(),
        });
    }

    // Page number
    if options.show_page_number {
        let x = page_size.x - margin_right;
        let y = (page_size.y - margin_bottom + FONT_SIZE).min(page_size.y - FONT_SIZE);
        texts.push(PrintableText {
            size: FONT_SIZE,
            pos: Vector2::new(x, y),
            angle: Rad(0.0),
            align: TextAlign::Far,
            text: format!("Page {}/{}", page + 1, page_count),
        });
    }

    // Edge IDs
    if options.edge_id_position != EdgeIdPosition::None {
        let in_page = options.is_in_page_fn(page);
        let edge_id_font_size = options.edge_id_font_size * 25.4 / 72.0; // pt to mm
        
        for (_i_island, island) in papercraft.islands() {
            // Island name
            let island_center = island.location();
            let (is_in, pos) = in_page(island_center);
            if is_in {
                texts.push(PrintableText {
                    size: edge_id_font_size,
                    pos,
                    angle: Rad(0.0),
                    align: TextAlign::Center,
                    text: island.name().to_string(),
                });
            }
        }
    }

    texts
}

/// Simple HTML escaping for text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================================
// PDF Generation
// ============================================================================

use lopdf::{
    Document, Object, Stream, StringFormat,
    content::{Content, Operation},
    dictionary,
    xref::XrefType,
};

/// Generate a PDF document from the papercraft project.
pub fn generate_pdf(papercraft: &Papercraft) -> Result<Vec<u8>> {
    let options = papercraft.options();
    let page_size_mm = Vector2::new(options.page_size.0, options.page_size.1);
    let page_count = options.pages;

    let mut doc = Document::with_version("1.4");
    doc.reference_table.cross_reference_type = XrefType::CrossReferenceTable;

    let id_pages = doc.new_object_id();

    let id_font = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });

    let mut pages = vec![];

    for page in 0..page_count {
        let ops = generate_pdf_page_ops(papercraft, page)?;
        
        let content = Content { operations: ops };
        let id_content = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let id_resources = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => id_font,
            },
        });
        
        let id_page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => id_pages,
            "Contents" => id_content,
            "Resources" => id_resources,
        });
        pages.push(id_page.into());
    }

    let pdf_pages = dictionary! {
        "Type" => "Pages",
        "Count" => pages.len() as i32,
        "Kids" => pages,
        "MediaBox" => vec![
            0.into(), 0.into(),
            (page_size_mm.x * 72.0 / 25.4).into(), (page_size_mm.y * 72.0 / 25.4).into()
        ],
    };
    doc.set_object(id_pages, pdf_pages);

    let id_catalog = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => id_pages,
    });
    doc.trailer.set("Root", id_catalog);

    // Metadata
    let date = time::OffsetDateTime::now_utc();
    let s_date = format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}Z",
        date.year(),
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute(),
        date.second(),
    );

    let id_info = doc.add_object(dictionary! {
        "Title" => Object::string_literal("Papercraft Export"),
        "Creator" => Object::string_literal(signature()),
        "CreationDate" => Object::string_literal(s_date.clone()),
        "ModDate" => Object::string_literal(s_date),
    });
    doc.trailer.set("Info", id_info);
    doc.compress();
    
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;
    Ok(buffer)
}

/// Generate PDF operations for a single page.
fn generate_pdf_page_ops(papercraft: &Papercraft, page: u32) -> Result<Vec<Operation>> {
    let options = papercraft.options();
    let page_size_mm = Vector2::new(options.page_size.0, options.page_size.1);
    let scale = options.scale;
    let in_page = options.is_in_page_fn(page);
    
    let mut ops: Vec<Operation> = Vec::new();

    // Helper to convert mm to points
    let mm_to_pt = |mm: f32| mm * 72.0 / 25.4;
    // PDF Y-coordinate is from bottom
    let pdf_y = |y: f32| (page_size_mm.y - y) * 72.0 / 25.4;

    // Get paper color
    let paper_color = &options.paper_color;
    
    // Draw faces as filled paths
    // Draw faces as filled paths
    // Draw faces as filled paths
    for (_i_island, island) in papercraft.islands() {
        let island_mx = island.matrix();
        let _ = papercraft.traverse_faces(island, |_i_face, face, mx| {
            let plane = papercraft.model().face_plane(face);
            let full_mx = island_mx * mx;
            
            let vertices: Vec<_> = face.index_vertices()
                .into_iter()
                .map(|i_v| {
                    let v = &papercraft.model()[i_v];
                    let p2d = plane.project(&v.pos(), scale);
                    full_mx.transform_point(Point2::from_vec(p2d)).to_vec()
                })
                .collect();
            
            // Check if any vertex is in page
            let any_in_page = vertices.iter().any(|v| in_page(*v).0);
            
            if any_in_page && vertices.len() >= 3 {
                // Set fill color
                ops.push(Operation::new(
                    "rg",
                    vec![paper_color.0.r.into(), paper_color.0.g.into(), paper_color.0.b.into()]
                ));
                
                // Move to first vertex
                let (_, p0) = in_page(vertices[0]);
                ops.push(Operation::new("m", vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()]));
                
                // Line to other vertices
                for v in &vertices[1..] {
                    let (_, p) = in_page(*v);
                    ops.push(Operation::new("l", vec![mm_to_pt(p.x).into(), pdf_y(p.y).into()]));
                }
                
                // Close and fill
                ops.push(Operation::new("f", vec![]));
            }
            
            ControlFlow::Continue(())
        });
    }

    // Draw cut lines (black)
    ops.push(Operation::new("RG", vec![0.0.into(), 0.0.into(), 0.0.into()]));
    ops.push(Operation::new("w", vec![0.5.into()])); // Line width

    for (i_island, island) in papercraft.islands() {
        // Build Face -> Island matrix map
        let mut face_matrices: std::collections::HashMap<crate::paper::FaceIndex, Matrix3> = std::collections::HashMap::new();
        let _ = papercraft.traverse_faces(island, |i_face, _, mx| {
            face_matrices.insert(i_face, *mx);
            ControlFlow::Continue(())
        });
        let island_mx = island.matrix();

        let perimeter = papercraft.island_perimeter(i_island);
        if perimeter.is_empty() {
            continue;
        }
        
        // Similar logic to SVG but output PDF path commands
        let mut contour_points: Vec<Vector2> = Vec::new();
        let mut touching = false;
        
        for peri in perimeter.iter() {
            let edge = &papercraft.model()[peri.i_edge()];
            let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
            let face = &papercraft.model()[i_face];
            let plane = papercraft.model().face_plane(face);
            
            let mx = face_matrices.get(&i_face).cloned().unwrap_or(Matrix3::from_scale(1.0));
            let full_mx = island_mx * mx;

            let (i_v0, _) = face.vertices_of_edge(peri.i_edge()).unwrap();
            let v0 = &papercraft.model()[i_v0];
            
            let p0_2d = plane.project(&v0.pos(), scale);
            let p0 = full_mx.transform_point(Point2::from_vec(p0_2d)).to_vec();
            
            let (is_in, pos) = in_page(p0);
            touching = touching || is_in;
            contour_points.push(pos);
        }
        
        if touching && !contour_points.is_empty() {
            let p0 = contour_points[0];
            ops.push(Operation::new("m", vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()]));
            
            for p in &contour_points[1..] {
                ops.push(Operation::new("l", vec![mm_to_pt(p.x).into(), pdf_y(p.y).into()]));
            }
            
            ops.push(Operation::new("s", vec![])); // Close and stroke
        }
    }

    // Draw text
    let texts = collect_texts(papercraft, page);
    if !texts.is_empty() {
        ops.push(Operation::new("BT", Vec::new()));
        
        for text in texts {
            let size = text.size * 72.0 / 25.4 / 1.1;
            ops.push(Operation::new("Tf", vec!["F1".into(), size.into()]));
            
            let x = mm_to_pt(text.pos.x);
            let y = pdf_y(text.pos.y);
            
            ops.push(Operation::new(
                "Tm",
                vec![1.0.into(), 0.0.into(), 0.0.into(), 1.0.into(), x.into(), y.into()]
            ));
            
            ops.push(Operation::new(
                "Tj",
                vec![Object::String(text.text.into_bytes(), StringFormat::Literal)]
            ));
        }
        
        ops.push(Operation::new("ET", Vec::new()));
    }

    Ok(ops)
}
