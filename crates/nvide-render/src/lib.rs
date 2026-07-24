//! Headless-friendly text layout/glyph mapping for the Phase 0 prototype.
//!
//! Full cosmic-text shaping comes later; this module maps rope buffer text to
//! monospaced glyph cells and CPU-side quads/atlas data the UI draws with wgpu.
//! Unit tests run without a display.

mod bitmap_font;
mod draw;

pub use bitmap_font::{build_atlas_r8, glyph_uv, ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PX};
pub use draw::{
    cells_to_quads, cells_to_vertices, default_cell_size, quads_to_vertices, GlyphQuad, GlyphVertex,
};

use nvide_buffer::{Buffer, RopeBuffer};

/// Clear color used by the empty-window path (sRGB 0–1).
pub const CLEAR_COLOR: [f64; 4] = [0.08, 0.09, 0.12, 1.0];

/// A single monospaced glyph cell produced from buffer text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphCell {
    /// Unicode scalar (replacement char for controls other than tab/newline).
    pub ch: char,
    /// Column in display cells (tab stops every `tab_width`).
    pub column: u32,
    /// Row (visual line index).
    pub row: u32,
}

/// Layout options for the monospaced prototype.
#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub tab_width: u32,
    /// Soft-wrap columns; 0 = no wrap.
    pub wrap_columns: u32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            tab_width: 4,
            wrap_columns: 0,
        }
    }
}

/// Map buffer text to glyph cells (consumes the shipped buffer API).
pub fn layout_buffer(buffer: &RopeBuffer, opts: LayoutOptions) -> Vec<GlyphCell> {
    layout_text(&buffer.to_string(), opts)
}

/// Map a string to glyph cells.
pub fn layout_text(text: &str, opts: LayoutOptions) -> Vec<GlyphCell> {
    let tab_width = opts.tab_width.max(1);
    let mut cells = Vec::new();
    let mut row: u32 = 0;
    let mut col: u32 = 0;

    for ch in text.chars() {
        match ch {
            '\n' => {
                row = row.saturating_add(1);
                col = 0;
            }
            '\r' => {}
            '\t' => {
                let advance = tab_width - (col % tab_width);
                for _ in 0..advance {
                    cells.push(GlyphCell {
                        ch: ' ',
                        column: col,
                        row,
                    });
                    col = col.saturating_add(1);
                    if opts.wrap_columns > 0 && col >= opts.wrap_columns {
                        row = row.saturating_add(1);
                        col = 0;
                    }
                }
            }
            c if c.is_control() => {
                cells.push(GlyphCell {
                    ch: '�',
                    column: col,
                    row,
                });
                col = col.saturating_add(1);
            }
            c => {
                cells.push(GlyphCell {
                    ch: c,
                    column: col,
                    row,
                });
                col = col.saturating_add(1);
                if opts.wrap_columns > 0 && col >= opts.wrap_columns {
                    row = row.saturating_add(1);
                    col = 0;
                }
            }
        }
    }
    cells
}

/// Visible rows occupied by a layout.
pub fn row_count(cells: &[GlyphCell]) -> u32 {
    cells
        .iter()
        .map(|c| c.row)
        .max()
        .map(|r| r + 1)
        .unwrap_or(1)
}

/// Build a simple ASCII preview of laid-out glyphs (for tests / headless logs).
pub fn preview_lines(cells: &[GlyphCell]) -> Vec<String> {
    let rows = row_count(cells) as usize;
    let mut lines = vec![String::new(); rows.max(1)];
    for cell in cells {
        let line = &mut lines[cell.row as usize];
        let col = cell.column as usize;
        if line.len() < col {
            line.push_str(&" ".repeat(col - line.len()));
        }
        if line.len() == col {
            line.push(cell.ch);
        } else if line.len() > col {
            // overwrite
            let mut chars: Vec<char> = line.chars().collect();
            if col < chars.len() {
                chars[col] = cell.ch;
                *line = chars.into_iter().collect();
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvide_buffer::{Buffer, BufferId, RopeBuffer};

    #[test]
    fn layout_from_rope_buffer() {
        let mut buf = RopeBuffer::new(BufferId(1));
        buf.insert(0, "ab\nc").unwrap();
        let cells = layout_buffer(&buf, LayoutOptions::default());
        assert_eq!(cells.len(), 3);
        assert_eq!(
            cells[0],
            GlyphCell {
                ch: 'a',
                column: 0,
                row: 0
            }
        );
        assert_eq!(
            cells[1],
            GlyphCell {
                ch: 'b',
                column: 1,
                row: 0
            }
        );
        assert_eq!(
            cells[2],
            GlyphCell {
                ch: 'c',
                column: 0,
                row: 1
            }
        );
        let lines = preview_lines(&cells);
        assert_eq!(lines, vec!["ab".to_string(), "c".to_string()]);
    }

    #[test]
    fn typing_updates_glyphs() {
        let mut buf = RopeBuffer::new(BufferId(1));
        for ch in ['H', 'i'] {
            let pos = buf.len_chars();
            buf.insert_tracked(pos, &ch.to_string()).unwrap();
        }
        let cells = layout_buffer(&buf, LayoutOptions::default());
        assert_eq!(preview_lines(&cells), vec!["Hi".to_string()]);
    }

    #[test]
    fn clear_color_is_opaque() {
        assert_eq!(CLEAR_COLOR[3], 1.0);
    }
}
