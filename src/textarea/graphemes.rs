use log::debug;
use ropey::iter::Chunks;
use ropey::RopeSlice;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// Length as grapheme count.
pub fn rope_len(r: RopeSlice<'_>) -> usize {
    let it = RopeGraphemes::new(r);
    it.filter(|c| c != "\n" && c != "\r\n").count()
}

/// Data for rendering/mapping graphemes to screen coordinates.
pub struct GDisplay<'a> {
    /// First char.
    pub glyph: Cow<'a, str>,
    /// Length for the glyph. Cells after the first are reset.
    pub len: usize,
}

impl<'a> Debug for GDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' len={}", self.glyph, self.len)
    }
}

pub struct GlyphIter<'a> {
    iter: RopeGraphemes<'a>,
    offset: usize,
    width: usize,
    tabs: usize,
    show_ctrl: bool,
    col: usize,
}

impl<'a> GlyphIter<'a> {
    pub fn new(
        slice: RopeSlice<'a>,
        offset: usize,
        width: usize,
        tabs: usize,
        show_ctrl: bool,
    ) -> Self {
        Self {
            iter: RopeGraphemes::new(slice),
            offset,
            width,
            tabs,
            show_ctrl,
            col: 0,
        }
    }
}

impl<'a> Iterator for GlyphIter<'a> {
    type Item = GDisplay<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(g) = self.iter.next() {
            let g = if let Some(g) = g.as_str() {
                Cow::Borrowed(g)
            } else {
                Cow::Owned(g.chars().collect::<String>())
            };

            let mut glyph;
            let mut len: usize;

            match g.as_ref() {
                "\n" | "\r\n" => {
                    len = if self.show_ctrl { 1 } else { 0 };
                    glyph = Cow::Borrowed(if self.show_ctrl { "\u{2424}" } else { "" });
                }
                "\t" => {
                    len = self.tabs - self.col % self.tabs;
                    glyph = Cow::Borrowed("\u{2409}");
                }
                c if ("\x00".."\x20").contains(&c) => {
                    static CCHAR: [&str; 32] = [
                        "\u{2400}", "\u{2401}", "\u{2402}", "\u{2403}", "\u{2404}", "\u{2405}",
                        "\u{2406}", "\u{2407}", "\u{2408}", "\u{2409}", "\u{240A}", "\u{240B}",
                        "\u{240C}", "\u{240D}", "\u{240E}", "\u{240F}", "\u{2410}", "\u{2411}",
                        "\u{2412}", "\u{2413}", "\u{2414}", "\u{2415}", "\u{2416}", "\u{2417}",
                        "\u{2418}", "\u{2419}", "\u{241A}", "\u{241B}", "\u{241C}", "\u{241D}",
                        "\u{241E}", "\u{241F}",
                    ];
                    let c0 = c.bytes().next().expect("byte");
                    len = 1;
                    glyph = Cow::Borrowed(if self.show_ctrl {
                        &CCHAR[c0 as usize]
                    } else {
                        "\u{FFFD}"
                    });
                }
                c => {
                    len = unicode_display_width::width(c) as usize;
                    glyph = g;
                }
            }

            let next_col = self.col + len;

            if self.col < self.offset {
                if self.col + len > self.offset {
                    glyph = Cow::Borrowed(" ");
                    len = self.offset - self.col;
                    self.col = next_col;
                    return Some(GDisplay { glyph, len });
                } else {
                    // out left
                    self.col = next_col;
                }
            } else if self.col < self.offset + self.width {
                if self.col + len > self.offset + self.width {
                    len = self.offset + self.width - self.col;
                    self.col = next_col;
                    return Some(GDisplay { glyph, len });
                } else {
                    self.col = next_col;
                    return Some(GDisplay { glyph, len });
                }
            } else {
                // out right, stop iter
                self.col = next_col;
                return None;
            }
        }
        None
    }
}

/// An implementation of a graphemes iterator, for iterating over
/// the graphemes of a RopeSlice.
#[derive(Debug)]
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    pub fn new(slice: RopeSlice<'a>) -> RopeGraphemes<'a> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: slice,
            chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(self.text.slice(a_char..b_char))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

/// An implementation of a graphemes iterator, for iterating over
/// the graphemes of a RopeSlice.
#[derive(Debug)]
pub struct RopeGraphemesIdx<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemesIdx<'a> {
    pub fn new(slice: RopeSlice<'a>) -> RopeGraphemesIdx<'a> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemesIdx {
            text: slice,
            chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemesIdx<'a> {
    type Item = ((usize, usize), RopeSlice<'a>);

    fn next(&mut self) -> Option<((usize, usize), RopeSlice<'a>)> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(((a, b), self.text.slice(a_char..b_char)))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some(((a, b), (&self.cur_chunk[a2..b2]).into()))
        }
    }
}
