use crate::_private::NonExhaustive;
use crate::event::PagerOutcome;
use crate::layout::StructuredLayout;
use crate::pager::{AreaHandle, PagerLayout, PagerStyle};
use crate::util::revert_style;
use rat_event::util::MouseFlagsN;
use rat_event::{ct_event, HandleEvent, MouseOnly, Regular};
use rat_focus::ContainerFlag;
use rat_reloc::RelocatableState;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::{Span, StatefulWidget, Style};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Widget};
use std::ops::Index;

/// Prepare the page-layout for your widgets.
///
/// This widget page-breaks the areas for your widgets
/// and allows to render them in a single column.
///
#[derive(Debug, Default, Clone)]
pub struct SinglePager<'a> {
    layout: PagerLayout,

    block: Option<Block<'a>>,
    style: Style,
    label_style: Option<Style>,
    nav_style: Option<Style>,
    title_style: Option<Style>,
}

/// Renders directly to the frame buffer.
///
/// * It maps your widget area from layout coordinates
///   to screen coordinates before rendering.
/// * It helps with cleanup of the widget state if your
///   widget is currently invisible.
#[derive(Debug)]
pub struct SinglePagerBuffer<'a> {
    layout: PagerLayout,

    // current page.
    page: usize,
    buffer: &'a mut Buffer,

    // inner area that will be rendered.
    widget_area: Rect,

    style: Style,
    label_style: Option<Style>,
    nav_style: Option<Style>,
}

/// Renders the finishings for the DualPager.
#[derive(Debug)]
pub struct SinglePagerWidget {
    style: Style,
    nav_style: Option<Style>,
}

/// Widget state.
#[derive(Debug, Clone)]
pub struct SinglePagerState {
    /// Full area for the widget.
    /// __read only__ renewed for each render.
    pub area: Rect,
    /// Area inside the border.
    /// __read only__ renewed for each render.
    pub widget_area: Rect,
    /// Title area except the page indicators.
    /// __read only__ renewed with each render
    pub scroll_area: Rect,
    /// Area for prev-page indicator.
    /// __read only__ renewed with each render.
    pub prev_area: Rect,
    /// Area for next-page indicator.
    /// __read only__ renewed with each render.
    pub next_area: Rect,

    /// Page layout
    /// __read only__ renewed with each render.
    pub layout: PagerLayout,

    /// Current page.
    /// __read+write__
    pub page: usize,

    /// This widget has no focus of its own, but this flag
    /// can be used to set a container state.
    pub c_focus: ContainerFlag,

    /// Mouse
    pub mouse: MouseFlagsN,

    /// Only construct with `..Default::default()`.
    pub non_exhaustive: NonExhaustive,
}

impl<'a> SinglePager<'a> {
    /// New SinglePage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Page layout.
    pub fn layout(mut self, page_layout: PagerLayout) -> Self {
        self.layout = page_layout;
        self
    }

    /// Set page layout from StructLayout
    pub fn struct_layout(mut self, page_layout: StructuredLayout) -> Self {
        self.layout = PagerLayout::with_layout(page_layout);
        self
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.block = self.block.map(|v| v.style(style));
        self
    }

    /// Style for navigation.
    pub fn nav_style(mut self, nav_style: Style) -> Self {
        self.nav_style = Some(nav_style);
        self
    }

    /// Style for the title.
    pub fn title_style(mut self, title_style: Style) -> Self {
        self.title_style = Some(title_style);
        self
    }

    /// Block for border
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block.style(self.style));
        self
    }

    /// Set all styles.
    pub fn styles(mut self, styles: PagerStyle) -> Self {
        self.style = styles.style;
        if let Some(label) = styles.label_style {
            self.label_style = Some(label);
        }
        if let Some(nav) = styles.nav {
            self.nav_style = Some(nav);
        }
        if let Some(title) = styles.title {
            self.title_style = Some(title);
        }
        if let Some(block) = styles.block {
            self.block = Some(block);
        }
        self.block = self.block.map(|v| v.style(styles.style));
        self
    }

    /// Calculate the layout width.
    pub fn layout_width(&self, area: Rect) -> u16 {
        self.inner(area).width
    }

    /// Calculate the view area.
    pub fn inner(&self, area: Rect) -> Rect {
        if let Some(block) = &self.block {
            block.inner(area)
        } else {
            Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            )
        }
    }

    /// Run the layout and create the second stage.
    pub fn into_buffer<'b>(
        self,
        area: Rect,
        buf: &'b mut Buffer,
        state: &mut SinglePagerState,
    ) -> SinglePagerBuffer<'b> {
        state.area = area;

        state.widget_area = if let Some(block) = &self.block {
            block.inner(area)
        } else {
            Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            )
        };

        let p1 = 5;
        let p4 = state.widget_area.width.saturating_sub(p1);
        state.prev_area = Rect::new(state.widget_area.x, area.y, p1, 1);
        state.next_area = Rect::new(state.widget_area.x + p4, area.y, p1, 1);
        state.scroll_area = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);

        // run page layout
        state.layout = self.layout;
        state.layout.layout(state.widget_area);
        // clip page nr
        state.set_page(state.page);

        // render
        let title = format!(" {}/{} ", state.page + 1, state.layout.num_pages());
        let block = self
            .block
            .unwrap_or_else(|| Block::new().borders(Borders::TOP).style(self.style))
            .title_bottom(title)
            .title_alignment(Alignment::Right);
        let block = if let Some(title_style) = self.title_style {
            block.title_style(title_style)
        } else {
            block
        };
        block.render(area, buf);

        SinglePagerBuffer {
            layout: state.layout.clone(),
            page: state.page,
            buffer: buf,
            widget_area: state.widget_area,
            style: self.style,
            label_style: self.label_style,
            nav_style: self.nav_style,
        }
    }
}

impl<'a> SinglePagerBuffer<'a> {
    /// Render a widget to the buffer.
    #[inline(always)]
    pub fn render_widget_area<W>(&mut self, widget: W, area: Rect)
    where
        W: Widget,
    {
        if let Some(buffer_area) = self.locate_area(area) {
            // render the actual widget.
            widget.render(buffer_area, self.buffer);
        } else {
            // noop
        }
    }

    /// Render a widget to the buffer.
    /// This expects that the state is a RelocatableState,
    /// so it can reset the areas for hidden widgets.
    #[inline(always)]
    pub fn render_area<W, S>(&mut self, widget: W, area: Rect, state: &mut S)
    where
        W: StatefulWidget<State = S>,
        S: RelocatableState,
    {
        if let Some(buffer_area) = self.locate_area(area) {
            // render the actual widget.
            widget.render(buffer_area, self.buffer, state);
        } else {
            self.hidden(state);
        }
    }

    /// Render the label if any.
    pub fn render_label(&mut self, area: AreaHandle) {
        if let Some(buffer_areas) = self.locate(area) {
            if let Some(label) = self.layout.label(area) {
                let style = self.label_style.unwrap_or(self.style);
                Span::from(label)
                    .style(style)
                    .render(buffer_areas[0], self.buffer);
            } else {
                Span::from("xxxx")
                    .white()
                    .on_red()
                    .render(buffer_areas[0], self.buffer);
            }
        }
    }

    /// Render a widget to the buffer.
    #[inline(always)]
    pub fn render_widget<W, Idx>(&mut self, widget: W, area: AreaHandle, tag: Idx)
    where
        W: Widget,
        [Rect]: Index<Idx, Output = Rect>,
    {
        if let Some(buffer_areas) = self.locate(area) {
            // render the actual widget.
            widget.render(buffer_areas[tag], self.buffer);
        } else {
            // noop
        }
    }

    /// Render a widget to the buffer.
    ///
    /// This expects that the state is a RelocatableState,
    /// so it can reset the areas for hidden widgets.
    #[inline(always)]
    pub fn render<W, S, Idx>(&mut self, widget: W, area: AreaHandle, tag: Idx, state: &mut S)
    where
        W: StatefulWidget<State = S>,
        S: RelocatableState,
        [Rect]: Index<Idx, Output = Rect>,
    {
        if let Some(buffer_areas) = self.locate(area) {
            // render the actual widget.
            widget.render(buffer_areas[tag], self.buffer, state);
        } else {
            self.hidden(state);
        }
    }

    /// Return the layout.
    pub fn layout(&self) -> &PagerLayout {
        &self.layout
    }

    /// Is the given area visible?
    pub fn is_visible_area(&self, area: Rect) -> bool {
        self.layout.buf_area(area).0 == self.page
    }

    /// Is the given area visible?
    pub fn is_visible(&self, handle: AreaHandle) -> bool {
        self.layout.buf_handle(handle).0 == self.page
    }

    /// Calculate the necessary shift from view to screen.
    /// This does nothing as pager always places the widgets
    /// in screen coordinates.
    ///
    /// Just to keep the api in sync with [Clipper].
    pub fn shift(&self) -> (i16, i16) {
        (0, 0)
    }

    /// Relocate an area from layout coordinates to screen coordinates.
    /// A result None indicates that the area is invisible.
    pub fn locate(&self, handle: AreaHandle) -> Option<Box<[Rect]>> {
        let (page, mut areas) = self.layout.buf_handle(handle);
        if self.page == page {
            for area in &mut areas {
                *area = Rect::new(
                    area.x + self.widget_area.x,
                    area.y + self.widget_area.y,
                    area.width,
                    area.height,
                );
            }
            Some(areas)
        } else {
            None
        }
    }

    /// Relocate an area from layout coordinates to screen coordinates.
    /// A result None indicates that the area is invisible.
    pub fn locate_area(&self, layout_area: Rect) -> Option<Rect> {
        let (page, area) = self.layout.buf_area(layout_area);
        if self.page == page {
            Some(Rect::new(
                area.x + self.widget_area.x,
                area.y + self.widget_area.y,
                area.width,
                area.height,
            ))
        } else {
            None
        }
    }

    /// Does nothing for pager.
    /// Just to keep the api in sync with [Clipper].
    pub fn relocate<S>(&self, _state: &mut S)
    where
        S: RelocatableState,
    {
    }

    /// Clear the areas in the widget-state.
    /// This is called by render_xx whenever a widget is invisible.
    pub fn hidden<S>(&self, state: &mut S)
    where
        S: RelocatableState,
    {
        state.relocate((0, 0), Rect::default())
    }

    /// Access the buffer.
    /// __Note__
    /// Use of render_xxx is preferred.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.buffer
    }

    /// Rendering the content is finished.
    ///
    /// Convert to the final widget to render the finishings.
    pub fn into_widget(self) -> SinglePagerWidget {
        SinglePagerWidget {
            style: self.style,
            nav_style: self.nav_style,
        }
    }
}

impl StatefulWidget for SinglePagerWidget {
    type State = SinglePagerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        assert_eq!(area, state.area);

        // active areas
        let nav_style = self.nav_style.unwrap_or(self.style);
        if matches!(state.mouse.hover.get(), Some(0)) {
            buf.set_style(state.prev_area, revert_style(nav_style));
        } else {
            buf.set_style(state.prev_area, nav_style);
        }
        if state.page > 0 {
            Span::from(" <<< ").render(state.prev_area, buf);
        } else {
            Span::from(" [·] ").render(state.prev_area, buf);
        }
        if matches!(state.mouse.hover.get(), Some(1)) {
            buf.set_style(state.next_area, revert_style(nav_style));
        } else {
            buf.set_style(state.next_area, nav_style);
        }
        if state.page + 1 < state.layout.num_pages() {
            Span::from(" >>> ").render(state.next_area, buf);
        } else {
            Span::from(" [·] ").render(state.next_area, buf);
        }
    }
}

impl Default for SinglePagerState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            widget_area: Default::default(),
            scroll_area: Default::default(),
            prev_area: Default::default(),
            next_area: Default::default(),
            layout: Default::default(),
            page: 0,
            c_focus: Default::default(),
            mouse: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl SinglePagerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the page for this rect.
    pub fn show(&mut self, handle: AreaHandle) {
        let (page, _) = self.layout.buf_handle(handle);
        self.page = page;
    }

    /// Show the page for this rect.
    pub fn show_area(&mut self, area: Rect) {
        let (page, _) = self.layout.buf_area(area);
        self.page = page;
    }

    /// First handle for the page.
    /// This returns the first handle for the page.
    /// Does not check whether the connected area is visible.
    pub fn first_handle(&self, page: usize) -> Option<AreaHandle> {
        self.layout.first_on_page(page)
    }

    /// Set the visible page.
    pub fn set_page(&mut self, page: usize) -> bool {
        let old_page = self.page;
        if page >= self.layout.num_pages() {
            self.page = self.layout.num_pages() - 1;
        } else {
            self.page = page;
        }
        old_page != self.page
    }

    /// Select next page. Keeps the page in bounds.
    pub fn next_page(&mut self) -> bool {
        let old_page = self.page;

        if self.page + 1 >= self.layout.num_pages() {
            self.page = self.layout.num_pages() - 1;
        } else {
            self.page += 1;
        }

        old_page != self.page
    }

    /// Select prev page.
    pub fn prev_page(&mut self) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }
}

impl HandleEvent<crossterm::event::Event, Regular, PagerOutcome> for SinglePagerState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> PagerOutcome {
        self.handle(event, MouseOnly)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, PagerOutcome> for SinglePagerState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> PagerOutcome {
        match event {
            ct_event!(mouse down Left for x,y) if self.prev_area.contains((*x, *y).into()) => {
                if self.prev_page() {
                    PagerOutcome::Page(self.page)
                } else {
                    PagerOutcome::Unchanged
                }
            }
            ct_event!(mouse down Left for x,y) if self.next_area.contains((*x, *y).into()) => {
                if self.next_page() {
                    PagerOutcome::Page(self.page)
                } else {
                    PagerOutcome::Unchanged
                }
            }
            ct_event!(scroll down for x,y) => {
                if self.scroll_area.contains((*x, *y).into()) {
                    if self.next_page() {
                        PagerOutcome::Page(self.page)
                    } else {
                        PagerOutcome::Unchanged
                    }
                } else {
                    PagerOutcome::Continue
                }
            }
            ct_event!(scroll up for x,y) => {
                if self.scroll_area.contains((*x, *y).into()) {
                    if self.prev_page() {
                        PagerOutcome::Page(self.page)
                    } else {
                        PagerOutcome::Unchanged
                    }
                } else {
                    PagerOutcome::Continue
                }
            }
            ct_event!(mouse any for m)
                if self.mouse.hover(&[self.prev_area, self.next_area], m) =>
            {
                PagerOutcome::Changed
            }
            _ => PagerOutcome::Continue,
        }
    }
}
