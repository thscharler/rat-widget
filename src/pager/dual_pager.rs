use crate::_private::NonExhaustive;
use crate::event::PagerOutcome;
use crate::pager::{AreaHandle, PageLayout, PagerStyle};
use crate::util::revert_style;
use rat_event::util::MouseFlagsN;
use rat_event::{ct_event, HandleEvent, MouseOnly, Regular};
use rat_focus::ContainerFlag;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::{Span, StatefulWidget, Style};
use ratatui::widgets::{Block, Borders, Widget};

/// Widget that displays one page of the PageLayout.
///
/// This only renders the navigation, you must render each widget
/// yourself. Call relocate(area) to get the actual screen-area
/// for your widget. If this call returns None, your widget shall
/// not be displayed.
#[derive(Debug, Default, Clone)]
pub struct DualPager<'a> {
    layout: PageLayout,
    style: Style,
    nav_style: Option<Style>,
    divider_style: Option<Style>,
    title_style: Option<Style>,
    block: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct RenderDualPager {
    inner1: Rect,
    divider: Rect,
    inner2: Rect,
    page: usize,
    layout: PageLayout,
    style: Style,
    divider_style: Option<Style>,
    nav_style: Option<Style>,
}

#[derive(Debug, Clone)]
pub struct DualPagerState {
    /// Full area.
    /// __read only__ renewed with each render.
    pub area: Rect,
    /// Area for widgets to render.
    /// __read only__ renewed with each render.
    pub widget_area1: Rect,
    /// Divider area.
    /// __read only__ renewed with each render.
    pub divider_area: Rect,
    /// Area for widgets to render.
    /// __read only__ renewed with each render.
    pub widget_area2: Rect,
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
    pub layout: PageLayout,
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

impl<'a> DualPager<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Page layout.
    pub fn layout(mut self, page_layout: PageLayout) -> Self {
        self.layout = page_layout;
        self
    }

    /// Set all styles.
    pub fn styles(mut self, styles: PagerStyle) -> Self {
        self.style = styles.style;
        if let Some(nav) = styles.nav {
            self.nav_style = Some(nav);
        }
        if let Some(divider) = styles.divider {
            self.divider_style = Some(divider);
        }
        if let Some(title) = styles.title {
            self.title_style = Some(title);
        }
        if let Some(block) = styles.block {
            self.block = Some(block);
        }
        self
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style for navigation.
    pub fn nav_style(mut self, nav_style: Style) -> Self {
        self.nav_style = Some(nav_style);
        self
    }

    /// Style for the divider.
    pub fn divider_style(mut self, divider_style: Style) -> Self {
        self.divider_style = Some(divider_style);
        self
    }

    /// Style for the title.
    pub fn title_style(mut self, title_style: Style) -> Self {
        self.title_style = Some(title_style);
        self
    }

    /// Block for border
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Returns the available width.
    pub fn get_width(&self, area: Rect) -> u16 {
        let inner = if let Some(block) = &self.block {
            block.inner(area)
        } else {
            Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            )
        };

        inner.width.saturating_sub(1) / 2
    }

    /// Run the layout and create the final Pager widget.
    pub fn into_widget(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut DualPagerState,
    ) -> RenderDualPager {
        state.area = area;

        let inner = if let Some(block) = &self.block {
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
        let p4 = inner.width - p1;
        state.prev_area = Rect::new(inner.x, area.y, p1, 1);
        state.next_area = Rect::new(inner.x + p4, area.y, p1, 1);
        state.scroll_area = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);

        let p1 = inner.width.saturating_sub(1) / 2;
        let p2 = inner.width.saturating_sub(1).saturating_sub(p1);
        state.widget_area1 = Rect::new(inner.x, inner.y, p1, inner.height);
        state.divider_area = Rect::new(inner.x + p1, inner.y, 1, inner.height);
        state.widget_area2 = Rect::new(inner.x + p1 + 1, inner.y, p2, inner.height);

        // run page layout
        state.layout = self.layout;
        state.layout.layout(state.widget_area1);
        // clip page nr
        state.set_page(state.page);

        // render
        buf.set_style(area, self.style);

        let title = format!(" {}/{} ", state.page + 1, state.layout.len());
        let block = self
            .block
            .unwrap_or_else(|| Block::new().borders(Borders::TOP))
            .title_bottom(title)
            .title_alignment(Alignment::Right);
        let block = if let Some(title_style) = self.title_style {
            block.title_style(title_style)
        } else {
            block
        };
        block.render(area, buf);

        RenderDualPager {
            inner1: state.widget_area1,
            divider: state.divider_area,
            inner2: state.widget_area2,
            page: state.page,
            layout: state.layout.clone(),
            style: self.style,
            divider_style: self.divider_style,
            nav_style: self.nav_style,
        }
    }
}

impl RenderDualPager {
    /// Relocate an area by handle from Layout coordinates to
    /// screen coordinates.
    ///
    /// A result None indicates that the area is
    /// out of view.
    pub fn relocate_handle(&self, handle: AreaHandle) -> Option<Rect> {
        let (page, target) = self.layout.locate_handle(handle);
        self._relocate(page, target)
    }

    /// Relocate a rect from Layout coordinates to
    /// screen coordinates.
    ///
    /// A result None indicates that the area is
    /// out of view.
    pub fn relocate(&self, area: Rect) -> Option<Rect> {
        let (page, target) = self.layout.locate(area);
        self._relocate(page, target)
    }

    fn _relocate(&self, page: usize, mut target_area: Rect) -> Option<Rect> {
        if self.page == page {
            target_area.x += self.inner1.x;
            target_area.y += self.inner1.y;
            Some(target_area)
        } else if self.page + 1 == page {
            target_area.x += self.inner2.x;
            target_area.y += self.inner2.y;
            Some(target_area)
        } else {
            None
        }
    }
}

impl StatefulWidget for RenderDualPager {
    type State = DualPagerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        assert_eq!(area, state.area);

        // divider
        let divider_style = self.divider_style.unwrap_or(self.style);
        if let Some(cell) = buf.cell_mut((self.divider.x, area.top())) {
            cell.set_style(divider_style);
            cell.set_symbol("\u{239E}");
        }
        for y in self.divider.top()..area.bottom().saturating_sub(1) {
            if let Some(cell) = buf.cell_mut((self.divider.x, y)) {
                cell.set_style(divider_style);
                cell.set_symbol("\u{239C}");
            }
        }
        if let Some(cell) = buf.cell_mut((self.divider.x, area.bottom().saturating_sub(1))) {
            cell.set_style(divider_style);
            cell.set_symbol("\u{239D}");
        }

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
        if state.page + 2 < state.layout.len() {
            Span::from(" >>> ").render(state.next_area, buf);
        } else {
            Span::from(" [·] ").render(state.next_area, buf);
        }
    }
}

impl Default for DualPagerState {
    fn default() -> Self {
        Self {
            area: Default::default(),
            widget_area1: Default::default(),
            divider_area: Default::default(),
            widget_area2: Default::default(),
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

impl DualPagerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the page for this rect.
    pub fn show_handle(&mut self, handle: AreaHandle) {
        let (page, _) = self.layout.locate_handle(handle);
        self.page = page & !1;
    }

    /// Show the page for this rect.
    pub fn show_area(&mut self, area: Rect) {
        let (page, _) = self.layout.locate(area);
        self.page = page & !1;
    }

    /// First handle for the page.
    /// This returns the first handle for the page.
    /// Does not check whether the connected area is visible.
    pub fn first_handle(&self, page: usize) -> Option<AreaHandle> {
        self.layout.first_handle(page)
    }

    /// Set the visible page.
    pub fn set_page(&mut self, page: usize) -> bool {
        let old_page = self.page;
        if page >= self.layout.len() {
            self.page = (self.layout.len() - 1) & !1;
        } else {
            self.page = page & !1;
        }
        old_page != self.page
    }

    /// Select next page. Keeps the page in bounds.
    pub fn next_page(&mut self) -> bool {
        let old_page = self.page;

        if self.page + 2 >= self.layout.len() {
            self.page = (self.layout.len() - 1) & !1;
        } else {
            self.page = (self.page + 2) & !1;
        }

        old_page != self.page
    }

    /// Select prev page.
    pub fn prev_page(&mut self) -> bool {
        if self.page >= 2 {
            self.page = (self.page - 2) & !1;
            true
        } else {
            false
        }
    }
}

impl HandleEvent<crossterm::event::Event, Regular, PagerOutcome> for DualPagerState {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> PagerOutcome {
        self.handle(event, MouseOnly)
    }
}

impl HandleEvent<crossterm::event::Event, MouseOnly, PagerOutcome> for DualPagerState {
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
