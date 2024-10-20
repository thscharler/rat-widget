//! A view allows scrolling of a `Widget` without builtin
//! support for scrolling.
//!
//! View and Viewport are the same in functionality.
//!
//! The difference is that View works for [Widget]s and
//! Viewport for [StatefulWidget]s.
//!

use crate::_private::NonExhaustive;
use crate::event::ScrollOutcome;
use crate::util::copy_buffer;
use crate::view::ViewStyle;
use rat_event::{HandleEvent, MouseOnly, Outcome};
use rat_scrolled::{Scroll, ScrollArea, ScrollAreaState, ScrollState};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::prelude::StatefulWidget;
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};
#[cfg(feature = "unstable-widget-ref")]
use ratatui::widgets::{StatefulWidgetRef, WidgetRef};

/// View has its own size, and can contain a stateless widget
/// that will be rendered to a view sized buffer.
/// This buffer is then offset and written to the actual
/// frame buffer.
#[derive(Debug, Default, Clone)]
pub struct View<'a, T> {
    widget: Option<T>,
    viewport: ViewImpl<'a>,
}

#[derive(Debug, Default, Clone)]
struct ViewImpl<'a> {
    block: Option<Block<'a>>,
    hscroll: Option<Scroll<'a>>,
    vscroll: Option<Scroll<'a>>,
    view_size: Size,
    style: Style,
}

/// State of the viewport.
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Complete area of the viewport.
    /// __readonly__. renewed for each render.
    pub area: Rect,
    /// Inner area of the viewport.
    /// __readonly__. renewed for each render.
    pub inner: Rect,
    /// The viewport area that the inner widget sees.
    /// __readonly__. renewed for each render.
    pub view: Rect,

    /// Horizontal scroll
    /// __read+write__
    pub hscroll: ScrollState,
    /// Vertical scroll
    /// __read+write__
    pub vscroll: ScrollState,

    /// Only construct with `..Default::default()`.
    pub non_exhaustive: NonExhaustive,
}

impl<'a, T> View<'a, T> {
    /// New viewport.
    pub fn new(inner: T) -> Self {
        Self {
            widget: Some(inner),
            viewport: Default::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.viewport.block = Some(block);
        self
    }

    pub fn scroll(mut self, scroll: Scroll<'a>) -> Self {
        self.viewport.hscroll = Some(scroll.clone().override_horizontal());
        self.viewport.vscroll = Some(scroll.override_vertical());
        self
    }

    pub fn hscroll(mut self, scroll: Scroll<'a>) -> Self {
        self.viewport.hscroll = Some(scroll.override_horizontal());
        self
    }

    pub fn vscroll(mut self, scroll: Scroll<'a>) -> Self {
        self.viewport.vscroll = Some(scroll.override_vertical());
        self
    }

    /// Size for the inner widget.
    pub fn view_size(mut self, size: Size) -> Self {
        self.viewport.view_size = size;
        self
    }

    /// Combined style.
    pub fn styles(mut self, styles: ViewStyle) -> Self {
        self.viewport.style = styles.style;
        if let Some(styles) = styles.scroll {
            self.viewport.hscroll = self.viewport.hscroll.map(|v| v.styles(styles.clone()));
            self.viewport.vscroll = self.viewport.vscroll.map(|v| v.styles(styles.clone()));
        }
        if styles.block.is_some() {
            self.viewport.block = styles.block;
        }
        self
    }

    /// Style for the empty space outside the rendered buffer.
    pub fn style(mut self, style: Style) -> Self {
        self.viewport.style = style;
        self
    }
}

#[cfg(feature = "unstable-widget-ref")]
impl<'a, T> StatefulWidgetRef for View<'a, T>
where
    T: WidgetRef,
{
    type State = ViewState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(w) = &self.widget {
            render_ref(
                &self.viewport,
                |area, buf| w.render_ref(area, buf),
                area,
                buf,
                state,
            );
        } else {
            unreachable!()
        }
    }
}

impl<'a, T> StatefulWidget for View<'a, T>
where
    T: Widget,
{
    type State = ViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(w) = self.widget.take() {
            render_ref(
                &self.viewport,
                |area, buf| w.render(area, buf),
                area,
                buf,
                state,
            );
        } else {
            unreachable!()
        }
    }
}

fn render_ref(
    viewport: &ViewImpl<'_>,
    render_widget: impl FnOnce(Rect, &mut Buffer),
    area: Rect,
    buf: &mut Buffer,
    state: &mut ViewState,
) {
    state.area = area;

    let sa = ScrollArea::new()
        .block(viewport.block.as_ref())
        .h_scroll(viewport.hscroll.as_ref())
        .v_scroll(viewport.vscroll.as_ref());
    state.inner = sa.inner(area, Some(&state.hscroll), Some(&state.vscroll));

    state.view = Rect::new(
        state.inner.x,
        state.inner.y,
        viewport.view_size.width,
        viewport.view_size.height,
    );

    state
        .hscroll
        .set_max_offset(state.view.width.saturating_sub(state.inner.width) as usize);
    state.hscroll.set_page_len(state.inner.width as usize);
    state
        .vscroll
        .set_max_offset(state.view.height.saturating_sub(state.inner.height) as usize);
    state.vscroll.set_page_len(state.inner.height as usize);

    sa.render(
        area,
        buf,
        &mut ScrollAreaState::new()
            .h_scroll(&mut state.hscroll)
            .v_scroll(&mut state.vscroll),
    );

    let mut tmp = Buffer::empty(state.view);
    render_widget(state.view, &mut tmp);

    copy_buffer(
        state.view,
        tmp,
        state.hscroll.offset(),
        state.vscroll.offset(),
        viewport.style,
        state.inner,
        buf,
    );
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            inner: Default::default(),
            view: Default::default(),
            hscroll: Default::default(),
            vscroll: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl ViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vertical_offset(&self) -> usize {
        self.vscroll.offset()
    }

    pub fn set_vertical_offset(&mut self, offset: usize) -> bool {
        let old = self.vscroll.offset();
        self.vscroll.set_offset(offset);
        old != self.vscroll.offset()
    }

    pub fn vertical_page_len(&self) -> usize {
        self.vscroll.page_len()
    }

    pub fn horizontal_offset(&self) -> usize {
        self.hscroll.offset()
    }

    pub fn set_horizontal_offset(&mut self, offset: usize) -> bool {
        let old = self.hscroll.offset();
        self.hscroll.set_offset(offset);
        old != self.hscroll.offset()
    }

    pub fn horizontal_page_len(&self) -> usize {
        self.hscroll.page_len()
    }

    pub fn horizontal_scroll_to(&mut self, pos: usize) -> bool {
        self.hscroll.scroll_to_pos(pos)
    }

    pub fn vertical_scroll_to(&mut self, pos: usize) -> bool {
        self.vscroll.scroll_to_pos(pos)
    }

    pub fn scroll_up(&mut self, delta: usize) -> bool {
        self.vscroll.scroll_up(delta)
    }

    pub fn scroll_down(&mut self, delta: usize) -> bool {
        self.vscroll.scroll_down(delta)
    }

    pub fn scroll_left(&mut self, delta: usize) -> bool {
        self.hscroll.scroll_left(delta)
    }

    pub fn scroll_right(&mut self, delta: usize) -> bool {
        self.hscroll.scroll_right(delta)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for ViewState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        let mut sas = ScrollAreaState::new()
            .area(self.inner)
            .h_scroll(&mut self.hscroll)
            .v_scroll(&mut self.vscroll);
        let r = match sas.handle(event, MouseOnly) {
            ScrollOutcome::Up(v) => self.scroll_up(v),
            ScrollOutcome::Down(v) => self.scroll_down(v),
            ScrollOutcome::Left(v) => self.scroll_left(v),
            ScrollOutcome::Right(v) => self.scroll_right(v),
            ScrollOutcome::VPos(v) => self.set_vertical_offset(v),
            ScrollOutcome::HPos(v) => self.set_horizontal_offset(v),
            ScrollOutcome::Continue => false,
            ScrollOutcome::Unchanged => false,
            ScrollOutcome::Changed => true,
        };
        if r {
            return Outcome::Changed;
        }

        Outcome::Continue
    }
}
