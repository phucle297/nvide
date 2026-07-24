//! CPU-side monospaced glyph draw list (fed to the GPU prototype).

use crate::bitmap_font::{glyph_uv, GLYPH_PX};
use crate::GlyphCell;

/// One textured quad in pixel coordinates (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphQuad {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// Interleaved vertex: pos.xy, uv.xy (clip-space filled by GPU path).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

/// Convert layout cells into screen-space quads.
pub fn cells_to_quads(
    cells: &[GlyphCell],
    origin_x: f32,
    origin_y: f32,
    cell_w: f32,
    cell_h: f32,
) -> Vec<GlyphQuad> {
    let mut quads = Vec::with_capacity(cells.len());
    for cell in cells {
        if cell.ch == ' ' {
            continue;
        }
        let x0 = origin_x + cell.column as f32 * cell_w;
        let y0 = origin_y + cell.row as f32 * cell_h;
        let (u0, v0, u1, v1) = glyph_uv(cell.ch);
        quads.push(GlyphQuad {
            x0,
            y0,
            x1: x0 + cell_w,
            y1: y0 + cell_h,
            u0,
            v0,
            u1,
            v1,
        });
    }
    quads
}

/// Expand quads to a triangle-list vertex buffer (6 verts per quad).
/// `viewport` is (width, height) in pixels; positions are converted to clip space.
pub fn quads_to_vertices(quads: &[GlyphQuad], viewport: (f32, f32)) -> Vec<GlyphVertex> {
    let (vw, vh) = viewport;
    let mut verts = Vec::with_capacity(quads.len() * 6);
    let to_clip = |x: f32, y: f32| -> [f32; 2] {
        let cx = (x / vw) * 2.0 - 1.0;
        let cy = 1.0 - (y / vh) * 2.0; // flip Y for top-left origin
        [cx, cy]
    };
    for q in quads {
        let p00 = to_clip(q.x0, q.y0);
        let p10 = to_clip(q.x1, q.y0);
        let p01 = to_clip(q.x0, q.y1);
        let p11 = to_clip(q.x1, q.y1);
        // tri 1: 00-10-11
        verts.push(GlyphVertex {
            pos: p00,
            uv: [q.u0, q.v0],
        });
        verts.push(GlyphVertex {
            pos: p10,
            uv: [q.u1, q.v0],
        });
        verts.push(GlyphVertex {
            pos: p11,
            uv: [q.u1, q.v1],
        });
        // tri 2: 00-11-01
        verts.push(GlyphVertex {
            pos: p00,
            uv: [q.u0, q.v0],
        });
        verts.push(GlyphVertex {
            pos: p11,
            uv: [q.u1, q.v1],
        });
        verts.push(GlyphVertex {
            pos: p01,
            uv: [q.u0, q.v1],
        });
    }
    verts
}

/// Default cell size matching the atlas glyph pixel size (scaled 2× for readability).
pub fn default_cell_size() -> (f32, f32) {
    let s = (GLYPH_PX * 2) as f32;
    (s, s)
}

/// Build GPU-ready vertices directly from layout cells.
pub fn cells_to_vertices(
    cells: &[GlyphCell],
    viewport: (f32, f32),
    origin: (f32, f32),
) -> Vec<GlyphVertex> {
    let (cw, ch) = default_cell_size();
    let quads = cells_to_quads(cells, origin.0, origin.1, cw, ch);
    quads_to_vertices(&quads, viewport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout_text, LayoutOptions};

    #[test]
    fn hi_produces_two_quads_and_twelve_vertices() {
        let cells = layout_text("Hi", LayoutOptions::default());
        let quads = cells_to_quads(&cells, 8.0, 8.0, 16.0, 16.0);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].x0, 8.0);
        assert_eq!(quads[1].x0, 24.0);
        let verts = quads_to_vertices(&quads, (960.0, 640.0));
        assert_eq!(verts.len(), 12);
        // First vertex of first glyph is near top-left → clip y positive
        assert!(verts[0].pos[1] > 0.0);
    }

    #[test]
    fn space_skipped() {
        let cells = layout_text("A B", LayoutOptions::default());
        let quads = cells_to_quads(&cells, 0.0, 0.0, 8.0, 8.0);
        assert_eq!(quads.len(), 2);
    }
}
