//! Pure vector PDF/SVG export module.
//!
//! Generates vector output without OpenGL or imgui dependencies.

use anyhow::Result;
use base64::prelude::*;
use cgmath::{EuclideanSpace, InnerSpace, Rad, SquareMatrix, Transform};
use std::io::Write;
use std::ops::ControlFlow;

use crate::paper::{
    signature, EdgeIdPosition, EdgeStatus, FlapStyle, FoldStyle, IslandKey, Papercraft,
};
use crate::util_3d::{Matrix3, Point2, Vector2};

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
pub fn generate_svg(papercraft: &Papercraft, page: u32, with_textures: bool) -> Result<String> {
    let mut output = Vec::new();
    write_svg_page(papercraft, page, with_textures, &mut output)?;
    Ok(String::from_utf8(output)?)
}

/// Generate a multi-page SVG (Inkscape-style with sodipodi:namedview).
pub fn generate_svg_multipage(papercraft: &Papercraft, with_textures: bool) -> Result<String> {
    let mut output = Vec::new();
    write_svg_multipage(papercraft, with_textures, &mut output)?;
    Ok(String::from_utf8(output)?)
}

/// Write a single SVG page to the given writer.
fn write_svg_page(
    papercraft: &Papercraft,
    page: u32,
    with_textures: bool,
    w: &mut impl Write,
) -> Result<()> {
    let options = papercraft.options();
    let page_size = Vector2::new(options.page_size.0, options.page_size.1);

    // SVG Header
    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#
    )?;
    writeln!(
        w,
        r#"<svg width="{0}mm" height="{1}mm" viewBox="0 0 {0} {1}" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
        page_size.x, page_size.y
    )?;

    // Write definitions (textures)
    let tex_dimensions = if with_textures {
        write_svg_defs(papercraft, w)?
    } else {
        Vec::new()
    };

    // Write all layers
    write_svg_layers(papercraft, page, with_textures, &tex_dimensions, w)?;

    writeln!(w, r#"</svg>"#)?;
    Ok(())
}

/// Write multi-page SVG.
fn write_svg_multipage(
    papercraft: &Papercraft,
    with_textures: bool,
    w: &mut impl Write,
) -> Result<()> {
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
    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#
    )?;
    writeln!(
        w,
        r#"<svg width="{0}mm" height="{1}mm" viewBox="0 0 {0} {1}" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
        total_width, total_height
    )?;

    // Write definitions (textures)
    let tex_dimensions = if with_textures {
        write_svg_defs(papercraft, w)?
    } else {
        Vec::new()
    };

    // Inkscape page definitions
    writeln!(w, r#"<sodipodi:namedview>"#)?;
    for p in 0..page_count {
        let page_offset = options.page_position(p);
        writeln!(
            w,
            r#"<inkscape:page x="{}" y="{}" width="{}" height="{}" id="Page_{}" />"#,
            page_offset.x,
            page_offset.y,
            page_size.x,
            page_size.y,
            p + 1
        )?;
    }
    writeln!(w, r#"</sodipodi:namedview>"#)?;

    // Write each page as a group
    for p in 0..page_count {
        let page_offset = options.page_position(p);
        writeln!(
            w,
            r#"<g inkscape:label="Page_{}" inkscape:groupmode="layer" id="page_{}" transform="translate({},{})">"#,
            p + 1,
            p + 1,
            page_offset.x,
            page_offset.y
        )?;
        write_svg_layers(papercraft, p, with_textures, &tex_dimensions, w)?;
        writeln!(w, r#"</g>"#)?;
    }

    writeln!(w, r#"</svg>"#)?;
    Ok(())
}

fn write_svg_defs(papercraft: &Papercraft, w: &mut impl Write) -> Result<Vec<(u32, u32)>> {
    writeln!(w, r#"<defs>"#)?;
    let mut tex_dimensions = Vec::new();
    for (i, texture) in papercraft.model().textures().enumerate() {
        if let Some(pixbuf) = texture.pixbuf() {
            let width = pixbuf.width();
            let height = pixbuf.height();
            tex_dimensions.push((width, height));

            let mut buf = Vec::new();
            pixbuf.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;
            let b64 = BASE64_STANDARD.encode(&buf);

            // Write the image with actual dimensions
            writeln!(
                w,
                r#"<image id="tex_{}" width="{}" height="{}" preserveAspectRatio="none" href="data:image/png;base64,{}" />"#,
                i, width, height, b64
            )?;
        } else {
            tex_dimensions.push((0, 0));
        }
    }
    writeln!(w, r#"</defs>"#)?;
    Ok(tex_dimensions)
}

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

/// Calculate the transform matrix to map the texture unit square to the face polygon.
/// Returns None if matrix is singular (degenerate triangle).
fn calc_texture_matrix(uvs: [Vector2; 3], pts: [Vector2; 3]) -> Option<Matrix3> {
    // We want M such that M * uv_i = pt_i for i=0..2
    // Using homogeneous coordinates:
    // U = [uv0_x uv1_x uv2_x]
    //     [uv0_y uv1_y uv2_y]
    //     [1     1     1    ]
    //
    // P = [pt0_x pt1_x pt2_x]
    //     [pt0_y pt1_y pt2_y]
    //     [1     1     1    ]
    //
    // M * U = P  =>  M = P * U^-1

    let u_mat = Matrix3::new(
        uvs[0].x, uvs[0].y, 1.0, uvs[1].x, uvs[1].y, 1.0, uvs[2].x, uvs[2].y, 1.0,
    );

    let p_mat = Matrix3::new(
        pts[0].x, pts[0].y, 1.0, pts[1].x, pts[1].y, 1.0, pts[2].x, pts[2].y, 1.0,
    );

    u_mat.invert().map(|u_inv| p_mat * u_inv)
}

/// Calculate SVG texture transform matrix from UVs to face vertices.
/// Maps pixel coordinates (UV * texture_size) to vertex coordinates.
/// Takes exactly 3 UV coordinates and 3 points (a triangle).
fn calc_svg_texture_matrix_triangle(
    uvs: [Vector2; 3],
    pts: [Vector2; 3],
    tex_width: u32,
    tex_height: u32,
) -> Option<Matrix3> {
    if tex_width == 0 || tex_height == 0 {
        return None;
    }

    let w = tex_width as f32;
    let h = tex_height as f32;

    // Convert UVs to pixel coordinates (like frontend does)
    // Also flip V coordinate: PDO uses V=0 at top, images have V=0 at bottom
    let pixel_uvs: [Vector2; 3] = [
        Vector2::new(uvs[0].x * w, (1.0 - uvs[0].y) * h),
        Vector2::new(uvs[1].x * w, (1.0 - uvs[1].y) * h),
        Vector2::new(uvs[2].x * w, (1.0 - uvs[2].y) * h),
    ];

    calc_texture_matrix(pixel_uvs, pts)
}

/// Write all SVG layers for a single page.
fn write_svg_layers(
    papercraft: &Papercraft,
    page: u32,
    with_textures: bool,
    tex_dimensions: &[(u32, u32)],
    w: &mut impl Write,
) -> Result<()> {
    let options = papercraft.options();
    let scale = options.scale;

    // Collect data
    // Collect data
    let mut faces_data: Vec<(
        IslandKey,
        crate::paper::FaceIndex,
        Vec<Vector2>,
        Option<usize>,
    )> = Vec::new();
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
        let mut face_matrices: std::collections::HashMap<crate::paper::FaceIndex, Matrix3> =
            std::collections::HashMap::new();
        let _ = papercraft.traverse_faces(island, |i_face, _, mx| {
            face_matrices.insert(i_face, *mx);
            ControlFlow::Continue(())
        });

        // 2. Collect Faces
        let _ = papercraft.traverse_faces(island, |i_face, face, full_mx| {
            let plane = papercraft.model().face_plane(face);

            let mut face_vertices: Vec<Vector2> = Vec::new();
            for i_vertex in face.index_vertices() {
                let vertex = &papercraft.model()[i_vertex];
                let v2d = plane.project(&vertex.pos(), scale);
                let transformed = full_mx.transform_point(Point2::from_vec(v2d)).to_vec();

                // Always convert to relative, no filtering
                face_vertices.push(transformed - page_offset);
            }

            // Get material index for texture lookup
            // Material index directly maps to texture index (0-based)
            let material_idx = usize::from(face.material());
            // Check if this material index has a valid texture with pixel data
            let texture_idx = papercraft
                .model()
                .textures()
                .nth(material_idx)
                .and_then(|t| {
                    if t.pixbuf().is_some() {
                        Some(material_idx)
                    } else {
                        None
                    }
                });

            faces_data.push((i_island, i_face, face_vertices, texture_idx));
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

                let full_mx = face_matrices
                    .get(&i_face)
                    .cloned()
                    .unwrap_or(Matrix3::identity());

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
                if edge_status != EdgeStatus::Joined {
                    continue;
                }

                let edge = &papercraft.model()[i_edge];
                let (_f_a, f_b_opt) = edge.faces();
                let Some(f_b) = f_b_opt else {
                    continue;
                };

                if i_face > f_b {
                    continue;
                }
                if i_face == f_b {
                    continue;
                }

                let plane = papercraft.model().face_plane(face);
                let Some((i_v0, i_v1)) = face.vertices_of_edge(i_edge) else {
                    continue;
                };

                let v0 = &papercraft.model()[i_v0];
                let v1 = &papercraft.model()[i_v1];

                let p0 = full_mx
                    .transform_point(Point2::from_vec(plane.project(&v0.pos(), scale)))
                    .to_vec();
                let p1 = full_mx
                    .transform_point(Point2::from_vec(plane.project(&v1.pos(), scale)))
                    .to_vec();

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
        // 5. Collect Flaps
        if options.flap_style != FlapStyle::None {
            for peri in papercraft.island_perimeter(i_island).iter() {
                let edge_status = papercraft.edge_status(peri.i_edge());
                if let EdgeStatus::Cut(flap_side) = edge_status {
                    if !flap_side.flap_visible(peri.face_sign()) {
                        continue;
                    }

                    if !flap_side.flap_visible(peri.face_sign()) {
                        continue;
                    }

                    let edge = &papercraft.model()[peri.i_edge()];
                    let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
                    let face = &papercraft.model()[i_face];
                    let plane = papercraft.model().face_plane(face);

                    let full_mx = face_matrices
                        .get(&i_face)
                        .cloned()
                        .unwrap_or(Matrix3::identity());

                    let Some((i_v0, i_v1)) = face.vertices_of_edge(peri.i_edge()) else {
                        continue;
                    };
                    let v0 = &papercraft.model()[i_v0];
                    let v1 = &papercraft.model()[i_v1];

                    let p0 = full_mx
                        .transform_point(Point2::from_vec(plane.project(&v0.pos(), scale)))
                        .to_vec();
                    let p1 = full_mx
                        .transform_point(Point2::from_vec(plane.project(&v1.pos(), scale)))
                        .to_vec();

                    let p0_rel = p0 - page_offset;
                    let p1_rel = p1 - page_offset;

                    // Calculation of flap geometry needs original vector direction,
                    // but we can use relative points since vector difference is same.
                    let edge_vec = p1_rel - p0_rel;
                    let edge_len = edge_vec.magnitude();
                    let normal = Vector2::new(-edge_vec.y, edge_vec.x).normalize();
                    let flap_width = options.flap_width.min(edge_len * 0.4);
                    let taper = 0.15;

                    let f0 =
                        p0_rel + normal * flap_width + edge_vec.normalize() * (edge_len * taper);
                    let f1 =
                        p1_rel + normal * flap_width - edge_vec.normalize() * (edge_len * taper);

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

    writeln!(
        w,
        r#"<g inkscape:label="Faces" inkscape:groupmode="layer" id="Faces">"#
    )?;
    for (idx, (_, face_idx, vertices, texture_idx)) in faces_data.iter().enumerate() {
        if vertices.len() >= 3 {
            let has_texture = with_textures && texture_idx.is_some();

            if has_texture {
                // Draw textured face with proper UV mapping using triangulation
                if let Some(tex_idx) = texture_idx {
                    // Get texture dimensions
                    let (tex_width, tex_height) =
                        tex_dimensions.get(*tex_idx).copied().unwrap_or((1, 1));

                    // Get UV coordinates from face vertices
                    let face_uvs: Vec<_> = papercraft.model()[*face_idx]
                        .index_vertices()
                        .into_iter()
                        .map(|i_v| papercraft.model()[i_v].uv())
                        .collect();

                    // Triangulate the face - each triangle gets its own transform
                    let triangles = triangulate_polygon(vertices.len());

                    for (tri_idx, tri_indices) in triangles.iter().enumerate() {
                        let tri_pts = [
                            vertices[tri_indices[0]],
                            vertices[tri_indices[1]],
                            vertices[tri_indices[2]],
                        ];
                        let tri_uvs = [
                            face_uvs[tri_indices[0]],
                            face_uvs[tri_indices[1]],
                            face_uvs[tri_indices[2]],
                        ];

                        // Calculate texture transform matrix for this triangle
                        if let Some(tex_matrix) = calc_svg_texture_matrix_triangle(
                            tri_uvs, tri_pts, tex_width, tex_height,
                        ) {
                            // Create clip path for this triangle
                            let clip_id = format!("clip_face_{}_{}", idx, tri_idx);
                            writeln!(w, r#"<defs>"#)?;
                            writeln!(w, r#"<clipPath id="{}">"#, clip_id)?;
                            writeln!(
                                w,
                                r#"<polygon points="{},{} {},{} {},{}"/>"#,
                                tri_pts[0].x,
                                tri_pts[0].y,
                                tri_pts[1].x,
                                tri_pts[1].y,
                                tri_pts[2].x,
                                tri_pts[2].y
                            )?;
                            writeln!(w, r#"</clipPath>"#)?;
                            writeln!(w, r#"</defs>"#)?;

                            // Clip the texture to this triangle
                            writeln!(w, r#"<g clip-path="url(#{})">"#, clip_id)?;

                            // Draw transformed texture image
                            let (a, b, c, d, e, f) = (
                                tex_matrix.x.x,
                                tex_matrix.x.y,
                                tex_matrix.y.x,
                                tex_matrix.y.y,
                                tex_matrix.z.x,
                                tex_matrix.z.y,
                            );
                            writeln!(
                                w,
                                "<use href=\"#tex_{}\" transform=\"matrix({} {} {} {} {} {})\"/>",
                                tex_idx, a, b, c, d, e, f
                            )?;

                            writeln!(w, r#"</g>"#)?;
                        }
                    }
                }
            } else {
                // Draw solid color face
                write!(
                    w,
                    r#"<polygon id="face_{}" fill="{}" stroke="none" points=""#,
                    idx, paper_color_hex
                )?;
                for v in vertices {
                    write!(w, "{},{} ", v.x, v.y)?;
                }
                writeln!(w, r#""/>"#)?;
            }
        }
    }
    writeln!(w, r#"</g>"#)?;

    // Write Flaps layer
    if !flap_polygons.is_empty() {
        writeln!(
            w,
            r#"<g inkscape:label="Flaps" inkscape:groupmode="layer" id="Flaps">"#
        )?;
        for (idx, vertices) in flap_polygons.iter().enumerate() {
            write!(
                w,
                r##"<polygon id="flap_{}" fill="#E0E0E0" stroke="{}" stroke-width="0.1" points=""##,
                idx, tab_color_hex
            )?;
            for v in vertices {
                write!(w, "{},{} ", v.x, v.y)?;
            }
            writeln!(w, r#""/>"#)?;
        }
        writeln!(w, r#"</g>"#)?;
    }

    // Write Fold lines layer
    if options.fold_style != FoldStyle::None {
        writeln!(
            w,
            r#"<g inkscape:label="Fold" inkscape:groupmode="layer" id="Fold">"#
        )?;
        writeln!(
            w,
            r#"<g inkscape:label="Mountain" inkscape:groupmode="layer" id="Mountain">"#
        )?;
        for (idx, (p0, p1)) in mountain_lines.iter().enumerate() {
            writeln!(
                w,
                r##"<line id="mountain_{}" x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.2" />"##,
                idx, p0.x, p0.y, p1.x, p1.y, fold_color_hex
            )?;
        }
        writeln!(w, r#"</g>"#)?;
        writeln!(
            w,
            r#"<g inkscape:label="Valley" inkscape:groupmode="layer" id="Valley">"#
        )?;
        for (idx, (p0, p1)) in valley_lines.iter().enumerate() {
            writeln!(
                w,
                r##"<line id="valley_{}" x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.2" stroke-dasharray="1,1"/>"##,
                idx, p0.x, p0.y, p1.x, p1.y, fold_color_hex
            )?;
        }
        writeln!(w, r#"</g></g>"#)?;
    }

    // Write Cut lines layer
    writeln!(
        w,
        r#"<g inkscape:label="Cut" inkscape:groupmode="layer" id="Cut">"#
    )?;
    for (idx, contour) in cut_paths.iter().enumerate() {
        write!(
            w,
            r##"<path id="cut_{}" fill="none" stroke="{}" stroke-width="0.3" d="M "##,
            idx, cut_color_hex
        )?;
        for (i, p) in contour.iter().enumerate() {
            if i == 0 {
                write!(w, "{},{} ", p.x, p.y)?;
            } else {
                write!(w, "L {},{} ", p.x, p.y)?;
            }
        }
        writeln!(w, r#"Z"/>"#)?;
    }
    writeln!(w, r#"</g>"#)?;

    // Write Text layer
    let texts = collect_texts(papercraft, options, page);
    if !texts.is_empty() {
        writeln!(
            w,
            r#"<g inkscape:label="Text" inkscape:groupmode="layer" id="Text">"#
        )?;
        for text in texts {
            let anchor = match text.align {
                TextAlign::Near => "",
                TextAlign::Center => "text-anchor:middle;",
                TextAlign::Far => "text-anchor:end;",
            };
            let angle_deg = text.angle.0.to_degrees();
            if angle_deg.abs() < 0.01 {
                writeln!(
                    w,
                    r#"<text x="{}" y="{}" style="{}font-size:{}px;font-family:sans-serif;fill:#000000">{}</text>"#,
                    text.pos.x,
                    text.pos.y,
                    anchor,
                    text.size,
                    html_escape(&text.text)
                )?;
            } else {
                writeln!(
                    w,
                    r#"<text x="{}" y="{}" style="{}font-size:{}px;font-family:sans-serif;fill:#000000" transform="rotate({} {} {})">{}</text>"#,
                    text.pos.x,
                    text.pos.y,
                    anchor,
                    text.size,
                    angle_deg,
                    text.pos.x,
                    text.pos.y,
                    html_escape(&text.text)
                )?;
            }
        }
        writeln!(w, r#"</g>"#)?;
    }

    Ok(())
}

/// Collect text elements for a page (page numbers, edge IDs, signature).
fn collect_texts(
    papercraft: &Papercraft,
    options: &crate::paper::PaperOptions,
    page: u32,
) -> Vec<PrintableText> {
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

use flate2::write::ZlibEncoder;
use flate2::Compression;
use lopdf::{
    content::{Content, Operation},
    dictionary,
    xref::XrefType,
    Document, Object, Stream, StringFormat,
};

/// Calculate the transform matrix to map texture UV coordinates to face polygon vertices.
/// Takes exactly 3 UV coordinates and 3 points (a triangle).
/// Returns None if matrix is singular (degenerate triangle).
fn calc_pdf_texture_matrix_triangle(uvs: [Vector2; 3], pts: [Point2; 3]) -> Option<Matrix3> {
    // We want M such that M * uv_i = pt_i for i=0..2
    // Using homogeneous coordinates:
    // U = [uv0_x uv1_x uv2_x]
    //     [uv0_y uv1_y uv2_y]
    //     [1     1     1    ]
    //
    // P = [pt0_x pt1_x pt2_x]
    //     [pt0_y pt1_y pt2_y]
    //     [1     1     1    ]
    //
    // M * U = P  =>  M = P * U^-1

    let u_mat = Matrix3::new(
        uvs[0].x, uvs[0].y, 1.0, uvs[1].x, uvs[1].y, 1.0, uvs[2].x, uvs[2].y, 1.0,
    );

    let p_mat = Matrix3::new(
        pts[0].x, pts[0].y, 1.0, pts[1].x, pts[1].y, 1.0, pts[2].x, pts[2].y, 1.0,
    );

    u_mat.invert().map(|u_inv| p_mat * u_inv)
}

/// Embed textures as XObject images in the PDF document.
/// Returns a vector of (ObjectId, width, height) for each texture.
/// Uses raw RGB data with FlateDecode compression (not PNG).
fn embed_pdf_textures(
    papercraft: &Papercraft,
    doc: &mut Document,
) -> Result<Vec<(lopdf::ObjectId, u32, u32)>> {
    let mut texture_info = Vec::new();

    for texture in papercraft.model().textures() {
        if let Some(pixbuf) = texture.pixbuf() {
            let width = pixbuf.width();
            let height = pixbuf.height();

            // Convert to RGB8 format (strip alpha if present)
            let rgb_image = pixbuf.to_rgb8();
            let raw_data = rgb_image.as_raw();

            // Compress raw RGB data with Flate/Zlib
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            std::io::Write::write_all(&mut encoder, raw_data)?;
            let compressed_data = encoder.finish()?;

            // Create image XObject with proper FlateDecode filter
            let image_dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width as i64,
                "Height" => height as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "FlateDecode",
            };

            let image_stream = Stream::new(image_dict, compressed_data);
            let id = doc.add_object(image_stream);
            texture_info.push((id, width, height));
        } else {
            // Push a placeholder if no texture data
            texture_info.push(((0, 0), 0, 0));
        }
    }

    Ok(texture_info)
}

/// Generate a PDF document from the papercraft project.
pub fn generate_pdf(papercraft: &Papercraft, with_textures: bool) -> Result<Vec<u8>> {
    let mut options = papercraft.options().clone();

    // Auto-detect page columns if islands extend beyond current cols
    // This prevents items placed visually in a horizontal row from being wrapped to the next row coordinates
    // if the page_cols setting is too low.
    const PAGE_SEP: f32 = 10.0;
    let max_col = papercraft
        .islands()
        .map(|(_, island)| {
            let (bb_min, bb_max) = papercraft.island_bounding_box_angle(island, Rad(0.0));
            let center = (bb_min + bb_max) / 2.0;
            (center.x / (options.page_size.0 + PAGE_SEP)) as i32
        })
        .max()
        .unwrap_or(0);

    if max_col >= options.page_cols as i32 {
        options.page_cols = (max_col + 1) as u32;
    }

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

    // Embed textures as XObjects if needed
    let texture_xobjects = if with_textures {
        embed_pdf_textures(papercraft, &mut doc)?
    } else {
        Vec::new()
    };

    let mut pages = vec![];

    for page in 0..page_count {
        let ops =
            generate_pdf_page_ops(papercraft, &options, page, with_textures, &texture_xobjects)?;

        let content = Content { operations: ops };
        let id_content = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        // Build resources dictionary with textures
        let mut resources = dictionary! {
            "Font" => dictionary! {
                "F1" => id_font,
            },
        };

        if !texture_xobjects.is_empty() {
            let xobj_dict: lopdf::Dictionary = texture_xobjects
                .iter()
                .enumerate()
                .filter(|(_, (id, _, _))| id.0 != 0) // Skip placeholders
                .map(|(i, (id, _, _))| (format!("Im{i}"), (*id).into()))
                .collect();
            resources.set("XObject", lopdf::Object::Dictionary(xobj_dict));
        }

        let id_resources = doc.add_object(resources);

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
fn generate_pdf_page_ops(
    papercraft: &Papercraft,
    options: &crate::paper::PaperOptions,
    page: u32,
    with_textures: bool,
    texture_xobjects: &[(lopdf::ObjectId, u32, u32)],
) -> Result<Vec<Operation>> {
    let page_size_mm = Vector2::new(options.page_size.0, options.page_size.1);
    let scale = options.scale;
    let page_offset = options.page_position(page);

    let mut ops: Vec<Operation> = Vec::new();

    // Helper to convert mm to points
    let mm_to_pt = |mm: f32| mm * 72.0 / 25.4;
    // PDF Y-coordinate is from bottom, relative to the current page top-left
    let pdf_y = |y: f32| (page_size_mm.y - y) * 72.0 / 25.4;

    // Get paper color
    let paper_color = &options.paper_color;

    // 1. Draw faces as filled paths
    for (_i_island, island) in papercraft.islands() {
        // Determine which page this island belongs to based on bounding box center
        let (bb_min, bb_max) = papercraft.island_bounding_box_angle(island, Rad(0.0));
        let center = (bb_min + bb_max) / 2.0;
        let po = options.global_to_page(center);
        let owner_page = (po.row as u32) * options.page_cols.max(1) + (po.col as u32);

        if owner_page != page {
            continue;
        }

        let _ = papercraft.traverse_faces(island, |_i_face, face, mx| {
            let plane = papercraft.model().face_plane(face);

            let vertices: Vec<_> = face
                .index_vertices()
                .into_iter()
                .map(|i_v| {
                    let v = &papercraft.model()[i_v];
                    let p2d = plane.project(&v.pos(), scale);
                    let p_global = mx.transform_point(Point2::from_vec(p2d)).to_vec();
                    p_global - page_offset
                })
                .collect();

            if vertices.len() >= 3 {
                // Get material index for this face (if any)
                // Material index directly maps to texture index (0-based)
                let material_idx = usize::from(face.material());
                // Check if this material index has a valid texture with pixel data
                let texture_info = texture_xobjects
                    .get(material_idx)
                    .filter(|(id, _, _)| id.0 != 0);
                let has_texture = with_textures && texture_info.is_some();

                // First, always draw the paper color fill as base
                ops.push(Operation::new(
                    "rg",
                    vec![
                        paper_color.0.r.into(),
                        paper_color.0.g.into(),
                        paper_color.0.b.into(),
                    ],
                ));

                // Move to first vertex
                let p0 = vertices[0];
                ops.push(Operation::new(
                    "m",
                    vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()],
                ));

                // Line to other vertices
                for p in &vertices[1..] {
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(p.x).into(), pdf_y(p.y).into()],
                    ));
                }

                // Close and fill
                ops.push(Operation::new("f", vec![]));

                // Draw texture if enabled and available
                if has_texture {
                    if let Some((_, _, _)) = texture_info {
                        // Get UV coordinates for this face
                        let uvs: Vec<_> = face
                            .index_vertices()
                            .into_iter()
                            .map(|i_v| {
                                let v = &papercraft.model()[i_v];
                                v.uv()
                            })
                            .collect();

                        if uvs.len() >= 3 {
                            // Triangulate the face - each triangle gets its own transform
                            let triangles = triangulate_polygon(vertices.len());

                            for tri_indices in triangles.iter() {
                                let tri_pts = [
                                    Point2::from_vec(vertices[tri_indices[0]]),
                                    Point2::from_vec(vertices[tri_indices[1]]),
                                    Point2::from_vec(vertices[tri_indices[2]]),
                                ];
                                let tri_uvs = [
                                    uvs[tri_indices[0]],
                                    uvs[tri_indices[1]],
                                    uvs[tri_indices[2]],
                                ];

                                // Calculate texture transform matrix for this triangle
                                if let Some(tex_matrix) =
                                    calc_pdf_texture_matrix_triangle(tri_uvs, tri_pts)
                                {
                                    // Save graphics state
                                    ops.push(Operation::new("q", vec![]));

                                    // Create clipping path with this triangle
                                    ops.push(Operation::new(
                                        "m",
                                        vec![
                                            mm_to_pt(tri_pts[0].x).into(),
                                            pdf_y(tri_pts[0].y).into(),
                                        ],
                                    ));
                                    ops.push(Operation::new(
                                        "l",
                                        vec![
                                            mm_to_pt(tri_pts[1].x).into(),
                                            pdf_y(tri_pts[1].y).into(),
                                        ],
                                    ));
                                    ops.push(Operation::new(
                                        "l",
                                        vec![
                                            mm_to_pt(tri_pts[2].x).into(),
                                            pdf_y(tri_pts[2].y).into(),
                                        ],
                                    ));
                                    ops.push(Operation::new("h", vec![])); // Close path
                                    ops.push(Operation::new("W", vec![])); // Set clipping path
                                    ops.push(Operation::new("n", vec![])); // End path without filling

                                    // The texture matrix maps UV pixel coords to paper coords (mm)
                                    // PDF images render as 1x1 unit square, so we need to:
                                    // 1. Scale by texture dimensions to get to pixel space
                                    // 2. Apply the UV-to-paper transform (in mm)
                                    // 3. Convert mm to points
                                    // 4. Handle PDF Y-axis inversion (origin at bottom-left)
                                    //
                                    // Matrix elements a,b,c,d are in mm/pixel units
                                    // Translation e,f are in mm
                                    // We need to convert all to points

                                    let a = tex_matrix.x.x;
                                    let b = tex_matrix.x.y;
                                    let c = tex_matrix.y.x;
                                    let d = tex_matrix.y.y;
                                    let e = tex_matrix.z.x;
                                    let f = tex_matrix.z.y;

                                    // Convert scale/rotation components from mm/pixel to pt/pixel
                                    let mm_to_pt_scale = 72.0 / 25.4;
                                    let a_pt = a * mm_to_pt_scale;
                                    let b_pt = -b * mm_to_pt_scale; // Negate for Y-flip
                                    let c_pt = c * mm_to_pt_scale;
                                    let d_pt = -d * mm_to_pt_scale; // Negate for Y-flip
                                    let e_pt = e * mm_to_pt_scale;
                                    let f_pt = (page_size_mm.y - f) * mm_to_pt_scale;

                                    // Apply UV-to-paper transformation matrix
                                    ops.push(Operation::new(
                                        "cm",
                                        vec![
                                            a_pt.into(),
                                            b_pt.into(),
                                            c_pt.into(),
                                            d_pt.into(),
                                            e_pt.into(),
                                            f_pt.into(),
                                        ],
                                    ));

                                    // Draw the texture image
                                    ops.push(Operation::new(
                                        "Do",
                                        vec![Object::Name(
                                            format!("Im{}", material_idx).into_bytes(),
                                        )],
                                    ));

                                    // Restore graphics state
                                    ops.push(Operation::new("Q", vec![]));
                                }
                            }
                        }
                    }
                }
            }

            ControlFlow::Continue(())
        });
    }

    // Draw lines (black)
    ops.push(Operation::new(
        "RG",
        vec![0.0.into(), 0.0.into(), 0.0.into()],
    ));
    ops.push(Operation::new("w", vec![0.5.into()])); // Line width

    for (i_island, island) in papercraft.islands() {
        // Bounding box filter
        let (bb_min, bb_max) = papercraft.island_bounding_box_angle(island, Rad(0.0));
        let center = (bb_min + bb_max) / 2.0;
        let po = options.global_to_page(center);
        let owner_page = (po.row as u32) * options.page_cols.max(1) + (po.col as u32);

        if owner_page != page {
            continue;
        }

        // Build Face -> Island matrix map for flaps and perimeter
        let mut face_matrices: std::collections::HashMap<crate::paper::FaceIndex, Matrix3> =
            std::collections::HashMap::new();
        let _ = papercraft.traverse_faces(island, |i_face, _, mx| {
            face_matrices.insert(i_face, *mx);
            ControlFlow::Continue(())
        });

        // 1. Draw Folds
        if options.fold_style != FoldStyle::None {
            let _ = papercraft.traverse_faces(island, |i_face, face, mx| {
                let plane = papercraft.model().face_plane(face);

                for i_edge in face.index_edges() {
                    let edge_status = papercraft.edge_status(i_edge);
                    if edge_status != EdgeStatus::Joined {
                        continue;
                    }

                    let edge = &papercraft.model()[i_edge];
                    let (_f_a, f_b_opt) = edge.faces();
                    let Some(f_b) = f_b_opt else {
                        continue;
                    };

                    if i_face > f_b {
                        continue;
                    }

                    let Some((i_v0, i_v1)) = face.vertices_of_edge(i_edge) else {
                        continue;
                    };
                    let v0 = &papercraft.model()[i_v0];
                    let v1 = &papercraft.model()[i_v1];

                    let p0_global = mx
                        .transform_point(Point2::from_vec(plane.project(&v0.pos(), scale)))
                        .to_vec();
                    let p1_global = mx
                        .transform_point(Point2::from_vec(plane.project(&v1.pos(), scale)))
                        .to_vec();

                    let p0 = p0_global - page_offset;
                    let p1 = p1_global - page_offset;

                    let angle = edge.angle().0;
                    if angle.is_sign_negative() {
                        // Valley: Dashed
                        ops.push(Operation::new(
                            "d",
                            vec![vec![2.into(), 2.into()].into(), 0.into()],
                        ));
                    } else {
                        // Mountain: Solid
                        ops.push(Operation::new("d", vec![vec![].into(), 0.into()]));
                    }
                    ops.push(Operation::new(
                        "m",
                        vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(p1.x).into(), pdf_y(p1.y).into()],
                    ));
                    ops.push(Operation::new("S", vec![]));
                }
                ControlFlow::Continue(())
            });
            // Reset dash
            ops.push(Operation::new("d", vec![vec![].into(), 0.into()]));
        }

        // 2. Draw Flaps
        if options.flap_style != FlapStyle::None {
            for peri in papercraft.island_perimeter(i_island).iter() {
                let edge_status = papercraft.edge_status(peri.i_edge());
                if let EdgeStatus::Cut(flap_side) = edge_status {
                    if !flap_side.flap_visible(peri.face_sign()) {
                        continue;
                    }

                    let edge = &papercraft.model()[peri.i_edge()];
                    let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
                    let face = &papercraft.model()[i_face];
                    let plane = papercraft.model().face_plane(face);

                    let mx = face_matrices
                        .get(&i_face)
                        .cloned()
                        .unwrap_or(Matrix3::identity());

                    let Some((i_v0, i_v1)) = face.vertices_of_edge(peri.i_edge()) else {
                        continue;
                    };
                    let v0 = &papercraft.model()[i_v0];
                    let v1 = &papercraft.model()[i_v1];

                    let p0_global = mx
                        .transform_point(Point2::from_vec(plane.project(&v0.pos(), scale)))
                        .to_vec();
                    let p1_global = mx
                        .transform_point(Point2::from_vec(plane.project(&v1.pos(), scale)))
                        .to_vec();

                    let p0 = p0_global - page_offset;
                    let p1 = p1_global - page_offset;

                    let edge_vec = p1 - p0;
                    let edge_len = edge_vec.magnitude();
                    let normal = Vector2::new(-edge_vec.y, edge_vec.x).normalize();
                    let flap_width = options.flap_width.min(edge_len * 0.4);
                    let taper = 0.15;

                    let f0 = p0 + normal * flap_width + edge_vec.normalize() * (edge_len * taper);
                    let f1 = p1 + normal * flap_width - edge_vec.normalize() * (edge_len * taper);

                    // Fill Flap
                    ops.push(Operation::new(
                        "rg",
                        vec![0.88.into(), 0.88.into(), 0.88.into()],
                    ));
                    ops.push(Operation::new(
                        "m",
                        vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(p1.x).into(), pdf_y(p1.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(f1.x).into(), pdf_y(f1.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(f0.x).into(), pdf_y(f0.y).into()],
                    ));
                    ops.push(Operation::new("f", vec![]));

                    // Stroke Flap
                    ops.push(Operation::new("w", vec![0.2.into()]));
                    ops.push(Operation::new(
                        "m",
                        vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(f0.x).into(), pdf_y(f0.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(f1.x).into(), pdf_y(f1.y).into()],
                    ));
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(p1.x).into(), pdf_y(p1.y).into()],
                    ));
                    ops.push(Operation::new("S", vec![]));
                }
            }
        }

        // 3. Draw Perimeter Cut Lines
        let perimeter = papercraft.island_perimeter(i_island);
        if !perimeter.is_empty() {
            let mut contour_points: Vec<Vector2> = Vec::new();

            for peri in perimeter.iter() {
                let edge = &papercraft.model()[peri.i_edge()];
                let i_face = edge.face_by_sign(peri.face_sign()).unwrap();
                let face = &papercraft.model()[i_face];
                let plane = papercraft.model().face_plane(face);

                let mx = face_matrices
                    .get(&i_face)
                    .cloned()
                    .unwrap_or(Matrix3::from_scale(1.0));

                let (i_v0, _) = face.vertices_of_edge(peri.i_edge()).unwrap();
                let v0 = &papercraft.model()[i_v0];

                let p0_2d = plane.project(&v0.pos(), scale);
                let p0_global = mx.transform_point(Point2::from_vec(p0_2d)).to_vec();

                contour_points.push(p0_global - page_offset);
            }

            if !contour_points.is_empty() {
                let p0 = contour_points[0];
                ops.push(Operation::new(
                    "m",
                    vec![mm_to_pt(p0.x).into(), pdf_y(p0.y).into()],
                ));

                for p in &contour_points[1..] {
                    ops.push(Operation::new(
                        "l",
                        vec![mm_to_pt(p.x).into(), pdf_y(p.y).into()],
                    ));
                }

                ops.push(Operation::new("s", vec![])); // Close and stroke
            }
        }
    }

    // Draw text
    let texts = collect_texts(papercraft, options, page);
    if !texts.is_empty() {
        ops.push(Operation::new("BT", Vec::new()));

        for text in texts {
            let size = text.size * 72.0 / 25.4 / 1.1;
            ops.push(Operation::new("Tf", vec!["F1".into(), size.into()]));

            // Heuristic alignment shift
            let mut x = mm_to_pt(text.pos.x);
            match text.align {
                TextAlign::Center => {
                    let approx_width = (text.text.len() as f32) * size * 0.5;
                    x -= approx_width / 2.0;
                }
                TextAlign::Far => {
                    let approx_width = (text.text.len() as f32) * size * 0.5;
                    x -= approx_width;
                }
                TextAlign::Near => {}
            }

            let y = pdf_y(text.pos.y);

            ops.push(Operation::new(
                "Tm",
                vec![
                    1.0.into(),
                    0.0.into(),
                    0.0.into(),
                    1.0.into(),
                    x.into(),
                    y.into(),
                ],
            ));

            ops.push(Operation::new(
                "Tj",
                vec![Object::String(
                    text.text.into_bytes(),
                    StringFormat::Literal,
                )],
            ));
        }

        ops.push(Operation::new("ET", Vec::new()));
    }

    Ok(ops)
}
