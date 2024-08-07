use ropey::iter::Chunks;
use ropey::RopeSlice;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::iter::once;
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete, Graphemes, UnicodeSegmentation};

/// Length as grapheme count, excluding line breaks.
pub(crate) fn rope_line_len(r: RopeSlice<'_>) -> usize {
    let it = RopeGraphemes::new(r);
    it.filter(|c| c != "\n" && c != "\r\n").count()
}

/// Length as grapheme count, excluding line breaks.
pub(crate) fn str_line_len(s: &str) -> usize {
    let it = s.graphemes(true);
    it.filter(|c| *c != "\n" && *c != "\r\n").count()
}

/// Length as char count, *including* line breaks.
pub(crate) fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Is the first char a whitespace
fn is_whitespace(s: &str) -> bool {
    s.chars()
        .next()
        .map(|v| v.is_whitespace())
        .unwrap_or_default()
}

/// Find the start of the next word. Word is everything that is not whitespace.
pub(crate) fn next_word_start(s: &str, mut pos: usize) -> usize {
    let mut it = s.graphemes(true);
    if pos > 0 {
        it.nth(pos - 1);
    }
    loop {
        let Some(c) = it.next() else {
            break;
        };
        if !is_whitespace(c) {
            break;
        }
        pos += 1;
    }

    pos
}

/// Find the end of the next word.  Skips whitespace first, then goes on
/// until it finds the next whitespace.
pub(crate) fn next_word_end(s: &str, mut pos: usize) -> usize {
    let mut it = s.graphemes(true);
    if pos > 0 {
        it.nth(pos - 1);
    }
    let mut init = true;
    loop {
        let Some(c) = it.next() else {
            break;
        };

        if init {
            if !is_whitespace(c) {
                init = false;
            }
        } else {
            if is_whitespace(c) {
                break;
            }
        }

        pos += 1;
    }

    pos
}

/// Find prev word. Skips whitespace first.
/// Attention: start/end are mirrored here compared to next_word_start/next_word_end,
/// both return start<=end!
pub(crate) fn prev_word_start(s: &str, pos: usize) -> usize {
    let mut it = s.graphemes(true);
    let len = str_line_len(s);
    let mut rpos = len - pos;
    if rpos > 0 {
        it.nth_back(rpos - 1);
    }
    let mut init = true;
    loop {
        let Some(c) = it.next_back() else {
            break;
        };

        if init {
            if !is_whitespace(c) {
                init = false;
            }
        } else {
            if is_whitespace(c) {
                break;
            }
        }

        rpos += 1;
    }

    len - rpos
}

/// Find the end of the previous word. Word is everything that is not whitespace.
/// Attention: start/end are mirrored here compared to next_word_start/next_word_end,
/// both return start<=end!
pub(crate) fn prev_word_end(s: &str, pos: usize) -> usize {
    let mut it = s.graphemes(true);
    let len = str_line_len(s);
    let mut rpos = len - pos;
    if rpos > 0 {
        it.nth_back(rpos - 1);
    }
    loop {
        let Some(c) = it.next_back() else {
            break;
        };
        if !is_whitespace(c) {
            break;
        }
        rpos += 1;
    }

    len - rpos
}

/// Is the position at a word boundary?
pub(crate) fn is_word_boundary(s: &str, pos: usize) -> bool {
    if pos == 0 {
        true
    } else {
        let mut it = s.graphemes(true);
        if let Some(c0) = it.nth(pos - 1) {
            if let Some(c1) = it.next() {
                is_whitespace(c0) && !is_whitespace(c1) || !is_whitespace(c0) && is_whitespace(c1)
            } else {
                true
            }
        } else {
            false
        }
    }
}

/// Find the start of the word at pos.
pub(crate) fn word_start(s: &str, pos: usize) -> usize {
    let mut it = s.graphemes(true);
    let len = str_line_len(s);
    let mut rpos = len - pos;
    if rpos > 0 {
        it.nth_back(rpos - 1);
    }
    loop {
        let Some(c) = it.next_back() else {
            break;
        };
        if is_whitespace(c) {
            break;
        }
        rpos += 1;
    }

    len - rpos
}

/// Find the end of the word at pos.
pub(crate) fn word_end(s: &str, mut pos: usize) -> usize {
    let mut it = s.graphemes(true);
    if pos > 0 {
        it.nth(pos - 1);
    }
    loop {
        let Some(c) = it.next() else {
            break;
        };
        if is_whitespace(c) {
            break;
        }
        pos += 1;
    }

    pos
}

/// Drop first graphem.
/// If s is empty do nothing.
pub(crate) fn drop_first(s: &str) -> &str {
    if s.is_empty() {
        s
    } else {
        split_at(s, 1).1
    }
}

/// Drop last graphem.
/// If s is empty do nothing.
pub(crate) fn drop_last(s: &str) -> &str {
    if s.is_empty() {
        s
    } else {
        let end = s.graphemes(true).count();
        split_at(s, end - 1).0
    }
}

/// Split selection for removal along the mask boundaries.
pub(crate) fn split_remove_mask(
    value: &str,
    selection: Range<usize>,
    mask: Range<usize>,
) -> (&str, &str, &str, &str, &str) {
    let mut byte_mask_start = None;
    let mut byte_mask_end = None;
    let mut byte_sel_start = None;
    let mut byte_sel_end = None;

    for (cidx, (idx, _c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == selection.start {
            byte_sel_start = Some(idx);
        }
        if cidx == selection.end {
            byte_sel_end = Some(idx);
        }
        if cidx == mask.start {
            byte_mask_start = Some(idx);
        }
        if cidx == mask.end {
            byte_mask_end = Some(idx);
        }
    }

    let byte_sel_start = if selection.start <= mask.start {
        byte_mask_start.expect("mask")
    } else if selection.start >= mask.end {
        byte_mask_end.expect("mask")
    } else {
        byte_sel_start.expect("mask")
    };

    let byte_sel_end = if selection.end <= mask.start {
        byte_mask_start.expect("mask")
    } else if selection.end >= mask.end {
        byte_mask_end.expect("mask")
    } else {
        byte_sel_end.expect("mask")
    };

    let byte_mask_start = byte_mask_start.expect("mask");
    let byte_mask_end = byte_mask_end.expect("mask");

    (
        &value[..byte_mask_start],
        &value[byte_mask_start..byte_sel_start],
        &value[byte_sel_start..byte_sel_end],
        &value[byte_sel_end..byte_mask_end],
        &value[byte_mask_end..],
    )
}

/// Split along mask-sections, search within the mask.
pub(crate) fn split_mask_match<'a>(
    value: &'a str,
    search: &str,
    mask: Range<usize>,
) -> (&'a str, &'a str, &'a str, &'a str, &'a str) {
    let mut byte_mask_start = None;
    let mut byte_mask_end = None;
    let mut byte_find_start = None;
    let mut byte_find_end = None;

    for (cidx, (idx, c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == mask.start {
            byte_mask_start = Some(idx);
        }
        if cidx >= mask.start && cidx < mask.end && c == search {
            byte_find_start = Some(idx);
            byte_find_end = Some(idx + c.len());
        }
        if cidx == mask.end {
            byte_mask_end = Some(idx);
        }
    }

    #[allow(clippy::unnecessary_unwrap)]
    let (byte_find_start, byte_find_end) = if byte_find_start.is_some() {
        (byte_find_start.expect("find"), byte_find_end.expect("find"))
    } else {
        (
            byte_mask_start.expect("mask"),
            byte_mask_start.expect("mask"),
        )
    };
    let byte_mask_start = byte_mask_start.expect("mask");
    let byte_mask_end = byte_mask_end.expect("mask");

    (
        &value[..byte_mask_start],
        &value[byte_mask_start..byte_find_start],
        &value[byte_find_start..byte_find_end],
        &value[byte_find_end..byte_mask_end],
        &value[byte_mask_end..],
    )
}

/// Split along mask bounds and again at the cursor.
pub(crate) fn split_mask(
    value: &str,
    cursor: usize,
    mask: Range<usize>,
) -> (&str, &str, &str, &str) {
    let mut byte_mask_start = None;
    let mut byte_mask_end = None;
    let mut byte_cursor = None;

    for (cidx, (idx, _c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == cursor {
            byte_cursor = Some(idx);
        }
        if cidx == mask.start {
            byte_mask_start = Some(idx);
        }
        if cidx == mask.end {
            byte_mask_end = Some(idx);
        }
    }

    let byte_cursor = if cursor <= mask.start {
        byte_mask_start.expect("mask")
    } else if cursor >= mask.end {
        byte_mask_end.expect("mask")
    } else {
        byte_cursor.expect("mask")
    };
    let byte_mask_start = byte_mask_start.expect("mask");
    let byte_mask_end = byte_mask_end.expect("mask");

    (
        &value[..byte_mask_start],
        &value[byte_mask_start..byte_cursor],
        &value[byte_cursor..byte_mask_end],
        &value[byte_mask_end..],
    )
}

pub(crate) fn split_at(value: &str, cursor: usize) -> (&str, &str) {
    let mut byte_cursor = None;

    for (cidx, (idx, _c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == cursor {
            byte_cursor = Some(idx);
        }
    }

    let byte_cursor = byte_cursor.expect("cursor");

    (&value[..byte_cursor], &value[byte_cursor..])
}

/// Split off selection
pub(crate) fn split3(value: &str, selection: Range<usize>) -> (&str, &str, &str) {
    let mut byte_selection_start = None;
    let mut byte_selection_end = None;

    for (cidx, (idx, _c)) in value
        .grapheme_indices(true)
        .chain(once((value.len(), "")))
        .enumerate()
    {
        if cidx == selection.start {
            byte_selection_start = Some(idx);
        }
        if cidx == selection.end {
            byte_selection_end = Some(idx)
        }
    }

    let byte_selection_start = byte_selection_start.expect("byte_selection_start_not_found");
    let byte_selection_end = byte_selection_end.expect("byte_selection_end_not_found");

    (
        &value[0..byte_selection_start],
        &value[byte_selection_start..byte_selection_end],
        &value[byte_selection_end..value.len()],
    )
}

/// Data for rendering/mapping graphemes to screen coordinates.
pub struct Glyph<'a> {
    /// First char.
    pub glyph: Cow<'a, str>,
    /// Length for the glyph.
    pub len: usize,
}

impl<'a> Debug for Glyph<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' len={}", self.glyph, self.len)
    }
}

/// Iterates a RopeSlice and returns graphemes + length as
/// [Glyph].
///
/// This is used for rendering text, and for mapping text-positions
/// to screen-positions and vice versa.
///
/// It
/// * has a length for the glyph. This is used for wide characters
///   and tab support.
/// * has a column-offset.
/// * can translate control-codes to visible graphemes.
#[derive(Debug)]
pub struct RopeGlyphIter<'a> {
    iter: RopeGraphemes<'a>,
    offset: usize,
    tabs: u16,
    show_ctrl: bool,
    col: usize,
}

impl<'a> RopeGlyphIter<'a> {
    /// New iterator.
    pub fn new(slice: RopeSlice<'a>) -> Self {
        Self {
            iter: RopeGraphemes::new(slice),
            offset: 0,
            tabs: 8,
            show_ctrl: false,
            col: 0,
        }
    }

    /// Text offset.
    /// Iterates only graphemes beyond this offset.
    /// Might return partial glyphs.
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Tab width
    pub fn set_tabs(&mut self, tabs: u16) {
        self.tabs = tabs;
    }

    /// Show ASCII control codes.
    pub fn set_show_ctrl(&mut self, show_ctrl: bool) {
        self.show_ctrl = show_ctrl;
    }
}

impl<'a> Iterator for RopeGlyphIter<'a> {
    type Item = Glyph<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(g) = self.iter.next() {
            let g = if let Some(g) = g.as_str() {
                Cow::Borrowed(g)
            } else {
                // crossing chunk boundaries must collect.
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
                    len = self.tabs as usize - self.col % self.tabs as usize;
                    glyph = Cow::Borrowed(if self.show_ctrl { "\u{2409}" } else { " " });
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

            // clip left
            if self.col < self.offset {
                if self.col + len > self.offset {
                    glyph = Cow::Borrowed(" ");
                    len = self.offset - self.col;
                    self.col = next_col;
                    return Some(Glyph { glyph, len });
                } else {
                    // out left
                    self.col = next_col;
                }
            } else {
                self.col = next_col;
                return Some(Glyph { glyph, len });
            }
        }

        None
    }
}

/// Iterates a RopeSlice and returns graphemes + length as
/// [Glyph].
///
/// This is used for rendering text, and for mapping text-positions
/// to screen-positions and vice versa.
///
/// It
/// * has a length for the glyph. This is used for wide characters
///   and tab support.
/// * has a column-offset.
/// * can translate control-codes to visible graphemes.
#[derive(Debug)]
pub struct GlyphIter<'a> {
    iter: Graphemes<'a>,
    offset: usize,
    show_ctrl: bool,
    col: usize,
}

impl<'a> GlyphIter<'a> {
    /// New iterator.
    pub fn new(slice: &'a str) -> Self {
        Self {
            iter: slice.graphemes(true),
            offset: 0,
            show_ctrl: false,
            col: 0,
        }
    }

    /// Text offset.
    /// Iterates only graphemes beyond this offset.
    /// Might return partial glyphs.
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Show ASCII control codes.
    pub fn set_show_ctrl(&mut self, show_ctrl: bool) {
        self.show_ctrl = show_ctrl;
    }
}

impl<'a> Iterator for GlyphIter<'a> {
    type Item = Glyph<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(g) = self.iter.next() {
            let mut glyph;
            let mut len: usize;

            match g.as_ref() {
                "\n" | "\r\n" => {
                    len = if self.show_ctrl { 1 } else { 0 };
                    glyph = if self.show_ctrl { "\u{2424}" } else { "" };
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
                    glyph = if self.show_ctrl {
                        &CCHAR[c0 as usize]
                    } else {
                        "\u{FFFD}"
                    };
                }
                c => {
                    len = unicode_display_width::width(c) as usize;
                    glyph = g;
                }
            }

            let next_col = self.col + len;

            // clip left
            if self.col < self.offset {
                if self.col + len > self.offset {
                    glyph = " ";
                    len = self.offset - self.col;
                    self.col = next_col;
                    return Some(Glyph {
                        glyph: Cow::Borrowed(glyph),
                        len,
                    });
                } else {
                    // out left
                    self.col = next_col;
                }
            } else {
                self.col = next_col;
                return Some(Glyph {
                    glyph: Cow::Borrowed(glyph),
                    len,
                });
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
    type Item = (Range<usize>, RopeSlice<'a>);

    fn next(&mut self) -> Option<(Range<usize>, RopeSlice<'a>)> {
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

            Some((a..b, self.text.slice(a_char..b_char)))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((a..b, (&self.cur_chunk[a2..b2]).into()))
        }
    }
}
