//!
//! Choice/Select widget.
//!
//! ```rust no_run
//! use rat_popup::Placement;
//! use rat_scrolled::Scroll;
//! use rat_widget::choice::{Choice, ChoiceState};
//! # use ratatui::prelude::*;
//! # use ratatui::widgets::Block;
//! # let mut buf = Buffer::default();
//! # let mut cstate = ChoiceState::default();
//! # let mut max_bounds: Rect = Rect::default();
//!
//! let (widget, popup) = Choice::new()
//!         .item(1, "Carrots")
//!         .item(2, "Potatoes")
//!         .item(3, "Onions")
//!         .item(4, "Peas")
//!         .item(5, "Beans")
//!         .item(6, "Tomatoes")
//!         .popup_block(Block::bordered())
//!         .popup_placement(Placement::AboveOrBelow)
//!         .popup_boundary(max_bounds)
//!         .into_widgets();
//!  widget.render(Rect::new(3,3,15,1), &mut buf, &mut cstate);
//!
//!  // ... render other widgets
//!
//!  popup.render(Rect::new(3,3,15,1), &mut buf, &mut cstate);
//!
//! ```
//!
use crate::_private::NonExhaustive;
use crate::util::{block_size, revert_style};
use rat_event::util::{item_at, mouse_trap, MouseFlags};
use rat_event::{ct_event, ConsumedEvent, HandleEvent, MouseOnly, Outcome, Popup, Regular};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus, Navigation};
use rat_popup::event::PopupOutcome;
use rat_popup::{Placement, PopupCore, PopupCoreState, PopupStyle};
use rat_reloc::{relocate_area, relocate_areas, RelocatableState};
use rat_scrolled::event::ScrollOutcome;
use rat_scrolled::{Scroll, ScrollAreaState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::BlockExt;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
#[cfg(feature = "unstable-widget-ref")]
use ratatui::widgets::StatefulWidgetRef;
use ratatui::widgets::{Block, StatefulWidget, Widget};
use std::cell::RefCell;
use std::cmp::{max, min};
use std::marker::PhantomData;
use std::rc::Rc;

/// Choice.
///
/// Select one of a list. No editable mode for this widget.
///
/// This doesn't render itself. [into_widgets](Choice::into_widgets)
/// creates the base part and the popup part, which are rendered
/// separately.
///
#[derive(Debug, Clone)]
pub struct Choice<'a, T>
where
    T: PartialEq,
{
    keys: Rc<RefCell<Vec<T>>>,
    items: Rc<RefCell<Vec<Line<'a>>>>,

    // Can return to default with a user interaction.
    default_key: Option<T>,

    style: Style,
    button_style: Option<Style>,
    select_style: Option<Style>,
    focus_style: Option<Style>,
    block: Option<Block<'a>>,

    popup_placement: Placement,
    popup_len: Option<u16>,
    popup: PopupCore<'a>,
}

/// Renders the main widget.
#[derive(Debug)]
pub struct ChoiceWidget<'a, T>
where
    T: PartialEq,
{
    keys: Rc<RefCell<Vec<T>>>,
    items: Rc<RefCell<Vec<Line<'a>>>>,

    // Can return to default with a user interaction.
    default_key: Option<T>,

    style: Style,
    button_style: Option<Style>,
    focus_style: Option<Style>,
    block: Option<Block<'a>>,
    len: Option<u16>,

    _phantom: PhantomData<T>,
}

/// Renders the popup. This is called after the rest
/// of the area is rendered and overwrites to display itself.
#[derive(Debug)]
pub struct ChoicePopup<'a, T>
where
    T: PartialEq,
{
    items: Rc<RefCell<Vec<Line<'a>>>>,

    style: Style,
    select_style: Option<Style>,

    popup_placement: Placement,
    popup_len: Option<u16>,
    popup: PopupCore<'a>,

    _phantom: PhantomData<T>,
}

/// Combined style.
#[derive(Debug, Clone)]
pub struct ChoiceStyle {
    pub style: Style,
    pub button: Option<Style>,
    pub select: Option<Style>,
    pub focus: Option<Style>,
    pub block: Option<Block<'static>>,

    pub popup: PopupStyle,
    pub popup_len: Option<u16>,

    pub non_exhaustive: NonExhaustive,
}

/// State.
#[derive(Debug)]
pub struct ChoiceState<T = usize>
where
    T: PartialEq,
{
    /// Total area.
    /// __read only__. renewed with each render.
    pub area: Rect,
    /// First char of each item for navigation.
    /// __read only__. renewed with each render.
    pub nav_char: Vec<Vec<char>>,
    /// Key for each item.
    /// __read only__. renewed with each render.
    pub keys: Vec<T>,
    /// Item area in the main widget.
    /// __read only__. renewed with each render.
    pub item_area: Rect,
    /// Button area in the main widget.
    /// __read only__. renewed with each render.
    pub button_area: Rect,
    /// Visible items in the popup.
    /// __read only__. renewed with each render.
    pub item_areas: Vec<Rect>,
    /// Can return to default with a user interaction.
    /// __read only__. renewed for each render.
    pub default_key: Option<T>,
    /// Select item.
    /// __read+write__
    pub selected: Option<usize>,
    /// Popup state.
    pub popup: PopupCoreState,

    /// Focus flag.
    /// __read+write__
    pub focus: FocusFlag,
    /// Mouse util.
    pub mouse: MouseFlags,

    pub non_exhaustive: NonExhaustive,
}

impl Default for ChoiceStyle {
    fn default() -> Self {
        Self {
            style: Default::default(),
            button: None,
            select: None,
            focus: None,
            block: None,
            popup: Default::default(),
            popup_len: None,
            non_exhaustive: NonExhaustive,
        }
    }
}

impl<T> Default for Choice<'_, T>
where
    T: PartialEq,
{
    fn default() -> Self {
        Self {
            keys: Default::default(),
            items: Default::default(),
            default_key: None,
            style: Default::default(),
            button_style: None,
            select_style: None,
            focus_style: None,
            block: None,
            popup_len: None,
            popup_placement: Placement::BelowOrAbove,
            popup: Default::default(),
        }
    }
}

impl<'a> Choice<'a, usize> {
    /// Add items with auto-generated keys.
    #[inline]
    pub fn auto_items<V: Into<Line<'a>>>(self, items: impl IntoIterator<Item = V>) -> Self {
        {
            let mut keys = self.keys.borrow_mut();
            let mut itemz = self.items.borrow_mut();

            keys.clear();
            itemz.clear();

            for (k, v) in items.into_iter().enumerate() {
                keys.push(k);
                itemz.push(v.into());
            }
        }

        self
    }

    /// Add an item with an auto generated key.
    pub fn auto_item(self, item: impl Into<Line<'a>>) -> Self {
        let idx = self.keys.borrow().len();
        self.keys.borrow_mut().push(idx);
        self.items.borrow_mut().push(item.into());
        self
    }
}

impl<'a, T> Choice<'a, T>
where
    T: PartialEq,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Button text.
    #[inline]
    pub fn items<V: Into<Line<'a>>>(self, items: impl IntoIterator<Item = (T, V)>) -> Self {
        {
            let mut keys = self.keys.borrow_mut();
            let mut itemz = self.items.borrow_mut();

            keys.clear();
            itemz.clear();

            for (k, v) in items.into_iter() {
                keys.push(k);
                itemz.push(v.into());
            }
        }

        self
    }

    /// Add an item.
    pub fn item(self, key: T, item: impl Into<Line<'a>>) -> Self {
        self.keys.borrow_mut().push(key);
        self.items.borrow_mut().push(item.into());
        self
    }

    /// Can return to default with user interaction.
    pub fn default_key(mut self, default: T) -> Self {
        self.default_key = Some(default);
        self
    }

    /// Combined styles.
    pub fn styles(mut self, styles: ChoiceStyle) -> Self {
        self.style = styles.style;
        if styles.button.is_some() {
            self.button_style = styles.button;
        }
        if styles.select.is_some() {
            self.select_style = styles.select;
        }
        if styles.focus.is_some() {
            self.focus_style = styles.focus;
        }
        if styles.block.is_some() {
            self.block = styles.block;
        }
        self.block = self.block.map(|v| v.style(self.style));
        if let Some(placement) = styles.popup.placement {
            self.popup_placement = placement;
        }
        if styles.popup_len.is_some() {
            self.popup_len = styles.popup_len;
        }
        self.popup = self.popup.styles(styles.popup);
        self
    }

    /// Base style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.block = self.block.map(|v| v.style(self.style));
        self
    }

    /// Style for the down button.
    pub fn button_style(mut self, style: Style) -> Self {
        self.button_style = Some(style);
        self
    }

    /// Selection in the list.
    pub fn select_style(mut self, style: Style) -> Self {
        self.select_style = Some(style);
        self
    }

    /// Focused style.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = Some(style);
        self
    }

    /// Block for the main widget.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self.block = self.block.map(|v| v.style(self.style));
        self
    }

    /// Placement of the popup.
    ///
    /// __Default__
    /// Default is BelowOrAbove.
    pub fn popup_placement(mut self, placement: Placement) -> Self {
        self.popup_placement = placement;
        self
    }

    /// Outer boundary for the popup.
    pub fn popup_boundary(mut self, boundary: Rect) -> Self {
        self.popup = self.popup.boundary(boundary);
        self
    }

    /// Override the popup length.
    ///
    /// __Default__
    /// Defaults to the number of items or 5.
    pub fn popup_len(mut self, len: u16) -> Self {
        self.popup_len = Some(len);
        self
    }

    /// Base style for the popup.
    pub fn popup_style(mut self, style: Style) -> Self {
        self.popup = self.popup.style(style);
        self
    }

    /// Block for the popup.
    pub fn popup_block(mut self, block: Block<'a>) -> Self {
        self.popup = self.popup.block(block);
        self
    }

    /// Scroll for the popup.
    pub fn popup_scroll(mut self, scroll: Scroll<'a>) -> Self {
        self.popup = self.popup.v_scroll(scroll);
        self
    }

    /// Adds an extra offset to the widget area.
    ///
    /// This can be used to
    /// * place the widget under the mouse cursor.
    /// * align the widget not by the outer bounds but by
    ///   the text content.
    pub fn popup_offset(mut self, offset: (i16, i16)) -> Self {
        self.popup = self.popup.offset(offset);
        self
    }

    /// Sets only the x offset.
    /// See [offset](Self::offset)
    pub fn popup_x_offset(mut self, offset: i16) -> Self {
        self.popup = self.popup.x_offset(offset);
        self
    }

    /// Sets only the y offset.
    /// See [offset](Self::offset)
    pub fn popup_y_offset(mut self, offset: i16) -> Self {
        self.popup = self.popup.y_offset(offset);
        self
    }

    /// Inherent width.
    pub fn width(&self) -> u16 {
        let w = self
            .items
            .borrow()
            .iter()
            .map(|v| v.width())
            .max()
            .unwrap_or_default();

        w as u16 + block_size(&self.block).width
    }

    /// Inherent height.
    pub fn height(&self) -> u16 {
        1 + block_size(&self.block).height
    }

    /// Choice itself doesn't render.
    ///
    /// This builds the widgets from the parameters set for Choice.
    pub fn into_widgets(self) -> (ChoiceWidget<'a, T>, ChoicePopup<'a, T>) {
        (
            ChoiceWidget {
                keys: self.keys,
                items: self.items.clone(),
                default_key: self.default_key,
                style: self.style,
                button_style: self.button_style,
                focus_style: self.focus_style,
                block: self.block,
                len: self.popup_len,
                _phantom: Default::default(),
            },
            ChoicePopup {
                items: self.items.clone(),
                style: self.style,
                select_style: self.select_style,
                popup: self.popup,
                popup_placement: self.popup_placement,
                popup_len: self.popup_len,
                _phantom: Default::default(),
            },
        )
    }
}

#[cfg(feature = "unstable-widget-ref")]
impl<'a, T> StatefulWidgetRef for ChoiceWidget<'a, T> {
    type State = ChoiceState<T>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_choice(self, area, buf, state);

        state.default_key = self.default_key.clone();
        state.keys = self.keys.borrow().clone();
    }
}

impl<T> StatefulWidget for ChoiceWidget<'_, T>
where
    T: PartialEq,
{
    type State = ChoiceState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_choice(&self, area, buf, state);

        state.default_key = self.default_key;
        state.keys = self.keys.take();
    }
}

fn render_choice<T: PartialEq>(
    widget: &ChoiceWidget<'_, T>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut ChoiceState<T>,
) {
    state.area = area;

    if !state.popup.is_active() {
        let len = widget
            .len
            .unwrap_or_else(|| min(5, widget.items.borrow().len()) as u16);
        state.popup.v_scroll.max_offset = widget.items.borrow().len().saturating_sub(len as usize);
        state.popup.v_scroll.page_len = len as usize;
        state
            .popup
            .v_scroll
            .scroll_to_pos(state.selected.unwrap_or_default());
    }

    state.nav_char.clear();
    state.nav_char.extend(widget.items.borrow().iter().map(|v| {
        v.spans
            .first()
            .and_then(|v| v.content.as_ref().chars().next())
            .map_or(Vec::default(), |c| c.to_lowercase().collect::<Vec<_>>())
    }));

    let inner = widget.block.inner_if_some(area);

    state.item_area = Rect::new(
        inner.x,
        inner.y,
        inner.width.saturating_sub(3),
        inner.height,
    );
    state.button_area = Rect::new(
        inner.right().saturating_sub(min(3, inner.width)),
        inner.y,
        3,
        inner.height,
    );

    let focus_style = widget.focus_style.unwrap_or(revert_style(widget.style));

    if state.is_focused() {
        if widget.block.is_some() {
            widget.block.render(area, buf);
        }
        buf.set_style(inner, focus_style);
    } else {
        if widget.block.is_some() {
            widget.block.render(area, buf);
        } else {
            buf.set_style(inner, widget.style);
        }
        if let Some(button_style) = widget.button_style {
            buf.set_style(state.button_area, button_style);
        }
    }

    if let Some(selected) = state.selected {
        if let Some(item) = widget.items.borrow().get(selected) {
            item.render(state.item_area, buf);
        }
    }

    let dy = if (state.button_area.height & 1) == 1 {
        state.button_area.height / 2
    } else {
        state.button_area.height.saturating_sub(1) / 2
    };
    let bc = if state.is_popup_active() {
        " ◆ "
    } else {
        " ▼ "
    };
    Span::from(bc).render(
        Rect::new(state.button_area.x, state.button_area.y + dy, 3, 1),
        buf,
    );
}

impl<T> StatefulWidget for ChoicePopup<'_, T>
where
    T: PartialEq,
{
    type State = ChoiceState<T>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_popup(&self, area, buf, state);
    }
}

fn render_popup<T: PartialEq>(
    widget: &ChoicePopup<'_, T>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut ChoiceState<T>,
) {
    if state.popup.is_active() {
        let len = widget
            .popup_len
            .unwrap_or_else(|| min(5, widget.items.borrow().len()) as u16);

        let popup_len = len + widget.popup.get_block_size().height;
        let popup_style = widget.popup.style;
        let pop_area = Rect::new(0, 0, area.width, popup_len);

        widget
            .popup
            .ref_constraint(widget.popup_placement.into_constraint(area))
            .render(pop_area, buf, &mut state.popup);

        let inner = state.popup.widget_area;

        state.popup.v_scroll.max_offset = widget
            .items
            .borrow()
            .len()
            .saturating_sub(inner.height as usize);
        state.popup.v_scroll.page_len = inner.height as usize;

        state.item_areas.clear();
        let mut row = inner.y;
        let mut idx = state.popup.v_scroll.offset;
        loop {
            if row >= inner.bottom() {
                break;
            }

            let item_area = Rect::new(inner.x, row, inner.width, 1);
            state.item_areas.push(item_area);

            if let Some(item) = widget.items.borrow().get(idx) {
                let style = if state.selected == Some(idx) {
                    widget.select_style.unwrap_or(revert_style(widget.style))
                } else {
                    popup_style
                };

                buf.set_style(item_area, style);
                item.render(item_area, buf);
            } else {
                // noop?
            }

            row += 1;
            idx += 1;
        }
    } else {
        state.popup.clear_areas();
    }
}

impl<T> Clone for ChoiceState<T>
where
    T: Clone + PartialEq,
{
    fn clone(&self) -> Self {
        Self {
            area: self.area,
            nav_char: self.nav_char.clone(),
            keys: self.keys.clone(),
            item_area: self.item_area,
            button_area: self.button_area,
            item_areas: self.item_areas.clone(),
            default_key: self.default_key.clone(),
            selected: self.selected,
            popup: self.popup.clone(),
            focus: FocusFlag::named(self.focus.name()),
            mouse: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl<T> Default for ChoiceState<T>
where
    T: PartialEq,
{
    fn default() -> Self {
        Self {
            area: Default::default(),
            nav_char: Default::default(),
            keys: Default::default(),
            item_area: Default::default(),
            button_area: Default::default(),
            item_areas: Default::default(),
            default_key: None,
            selected: None,
            popup: Default::default(),
            focus: Default::default(),
            mouse: Default::default(),
            non_exhaustive: NonExhaustive,
        }
    }
}

impl<T> HasFocus for ChoiceState<T>
where
    T: PartialEq,
{
    fn build(&self, builder: &mut FocusBuilder) {
        builder.add_widget(self.focus(), self.area(), 0, self.navigable());
        builder.add_widget(self.focus(), self.popup.area, 1, Navigation::Mouse);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}

impl<T> RelocatableState for ChoiceState<T>
where
    T: PartialEq,
{
    fn relocate(&mut self, shift: (i16, i16), clip: Rect) {
        self.area = relocate_area(self.area, shift, clip);
        self.item_area = relocate_area(self.item_area, shift, clip);
        self.button_area = relocate_area(self.button_area, shift, clip);
        relocate_areas(&mut self.item_areas, shift, clip);
        self.popup.relocate(shift, clip);
    }
}

impl<T> ChoiceState<T>
where
    T: PartialEq,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn named(name: &str) -> Self {
        Self {
            focus: FocusFlag::named(name),
            ..Default::default()
        }
    }

    /// Popup is active?
    pub fn is_popup_active(&self) -> bool {
        self.popup.is_active()
    }

    /// Flip the popup state.
    pub fn flip_popup_active(&mut self) {
        self.popup.flip_active();
    }

    /// Show the popup.
    pub fn set_popup_active(&mut self, active: bool) -> bool {
        let old_active = self.popup.is_active();
        self.popup.set_active(active);
        old_active != active
    }

    /// Set the default value.
    ///
    /// Returns false if there is no default value, or
    /// no items, or nothing changed.
    ///
    /// Doesn't change the selection if the default key doesn't exist.
    pub fn set_default_value(&mut self) -> bool {
        let old_selected = self.selected;

        if let Some(default_key) = &self.default_key {
            for (i, k) in self.keys.iter().enumerate() {
                if default_key == k {
                    self.selected = Some(i);
                    return old_selected != self.selected;
                }
            }
        }
        old_selected != self.selected
    }

    /// Select the given value.
    ///
    /// Returns false if there is no such value, or
    /// no items, or nothing changed.
    ///
    /// Doesn't change the selection if the given key doesn't exist.
    pub fn set_value(&mut self, key: &T) -> bool
    where
        T: PartialEq,
    {
        let old_selected = self.selected;
        for (i, k) in self.keys.iter().enumerate() {
            if key == k {
                self.selected = Some(i);
                return old_selected != self.selected;
            }
        }
        old_selected != self.selected
    }

    /// Get the selected value or None if no value
    /// is selected or there are no items.
    pub fn value_opt_ref(&self) -> Option<&T> {
        if let Some(selected) = self.selected {
            Some(&self.keys[selected])
        } else {
            None
        }
    }

    /// Get the selected value.
    ///
    /// Panics if there is no selection or no items.
    pub fn value_ref(&self) -> &T {
        &self.keys[self.selected.expect("selection")]
    }

    /// Select
    pub fn select(&mut self, select: Option<usize>) -> bool {
        let old_selected = self.selected;

        if self.keys.is_empty() {
            self.selected = None;
        } else {
            if let Some(select) = select {
                self.selected = Some(select.clamp(0, self.keys.len() - 1));
            } else {
                self.selected = None;
            }
        }

        old_selected != self.selected
    }

    /// Selected
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Items?
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Scroll offset for the item list.
    pub fn clear_offset(&mut self) {
        self.popup.v_scroll.set_offset(0);
    }

    /// Scroll offset for the item list.
    pub fn set_offset(&mut self, offset: usize) -> bool {
        self.popup.v_scroll.set_offset(offset)
    }

    /// Scroll offset for the item list.
    pub fn offset(&self) -> usize {
        self.popup.v_scroll.offset()
    }

    /// Scroll offset for the item list.
    pub fn max_offset(&self) -> usize {
        self.popup.v_scroll.max_offset()
    }

    /// Page length for the item list.
    pub fn page_len(&self) -> usize {
        self.popup.v_scroll.page_len()
    }

    /// Scroll unit for the item list.
    pub fn scroll_by(&self) -> usize {
        self.popup.v_scroll.scroll_by()
    }

    /// Scroll the item list to the selected value.
    pub fn scroll_to_selected(&mut self) -> bool {
        if let Some(selected) = self.selected {
            self.popup.v_scroll.scroll_to_pos(selected)
        } else {
            false
        }
    }
}

impl<T> ChoiceState<T>
where
    T: PartialEq + Clone,
{
    /// Get the selected value or None if no value
    /// is selected or there are no items.
    pub fn value_opt(&self) -> Option<T> {
        if let Some(selected) = self.selected {
            Some(self.keys[selected].clone())
        } else {
            None
        }
    }

    /// Get the selected value.
    ///
    /// Panics if there is no selection or no items.
    pub fn value(&self) -> T {
        self.keys[self.selected.expect("selection")].clone()
    }
}

impl<T> ChoiceState<T>
where
    T: PartialEq,
{
    /// Select by first character.
    pub fn select_by_char(&mut self, c: char) -> bool {
        if self.nav_char.is_empty() {
            return false;
        }

        let selected = self.selected.unwrap_or_default();

        let c = c.to_lowercase().collect::<Vec<_>>();
        let mut idx = selected + 1;
        loop {
            if idx >= self.nav_char.len() {
                idx = 0;
            }
            if idx == selected {
                break;
            }

            if self.nav_char[idx] == c {
                self.selected = Some(idx);
                return true;
            }

            idx += 1;
        }
        false
    }

    /// Select at position
    pub fn move_to(&mut self, n: usize) -> bool {
        let r1 = self.select(Some(n));
        let r2 = self.scroll_to_selected();
        r1 || r2
    }

    /// Select next entry.
    pub fn move_down(&mut self, n: usize) -> bool {
        let old_selected = self.selected;

        if self.keys.is_empty() {
            self.selected = None;
        } else {
            if let Some(selected) = self.selected {
                self.selected = Some((selected + n).clamp(0, self.keys.len() - 1));
            } else {
                self.selected = Some(0);
            }
        }

        let r2 = self.scroll_to_selected();

        old_selected != self.selected || r2
    }

    /// Select prev entry.
    pub fn move_up(&mut self, n: usize) -> bool {
        let old_selected = self.selected;

        if self.keys.is_empty() {
            self.selected = None;
        } else {
            if let Some(selected) = self.selected {
                self.selected = Some(selected.saturating_sub(n).clamp(0, self.keys.len() - 1));
            } else {
                self.selected = Some(self.keys.len() - 1);
            }
        }

        let r2 = self.scroll_to_selected();

        old_selected != self.selected || r2
    }
}

impl<T: PartialEq> HandleEvent<crossterm::event::Event, Regular, Outcome> for ChoiceState<T> {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Regular) -> Outcome {
        // todo: here???
        let r0 = if self.lost_focus() {
            self.set_popup_active(false);
            Outcome::Changed
        } else {
            Outcome::Continue
        };

        let r1 = if self.is_focused() {
            match event {
                ct_event!(key press ' ') => {
                    self.flip_popup_active();
                    Outcome::Changed
                }
                ct_event!(key press c) => {
                    if self.select_by_char(*c) {
                        self.scroll_to_selected();
                        Outcome::Changed
                    } else {
                        Outcome::Unchanged
                    }
                }
                ct_event!(keycode press Enter) | ct_event!(keycode press Esc) => {
                    self.set_popup_active(false).into()
                }
                ct_event!(keycode press Delete) | ct_event!(keycode press Backspace) => {
                    if self.default_key.is_some() {
                        self.set_default_value();
                        Outcome::Changed
                    } else {
                        Outcome::Continue
                    }
                }
                ct_event!(keycode press Down) => {
                    let r0 = if !self.popup.is_active() {
                        self.popup.set_active(true);
                        Outcome::Changed
                    } else {
                        Outcome::Continue
                    };
                    let r1 = self.move_down(1).into();
                    max(r0, r1)
                }
                ct_event!(keycode press Up) => {
                    let r0 = if !self.popup.is_active() {
                        self.popup.set_active(true);
                        Outcome::Changed
                    } else {
                        Outcome::Continue
                    };
                    let r1 = self.move_up(1).into();
                    max(r0, r1)
                }
                _ => Outcome::Continue,
            }
        } else {
            Outcome::Continue
        };

        let r1 = if !r1.is_consumed() {
            self.handle(event, MouseOnly)
        } else {
            r1
        };

        max(r0, r1)
    }
}

impl<T: PartialEq> HandleEvent<crossterm::event::Event, MouseOnly, Outcome> for ChoiceState<T> {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: MouseOnly) -> Outcome {
        let r = match event {
            ct_event!(mouse down Left for x,y)
                if self.item_area.contains((*x, *y).into())
                    || self.button_area.contains((*x, *y).into()) =>
            {
                if !self.gained_focus() && !self.is_popup_active() && !self.popup.active.lost() {
                    self.set_popup_active(true);
                    Outcome::Changed
                } else {
                    // hide is down by self.popup.handle() as this click
                    // is outside the popup area!!
                    Outcome::Continue
                }
            }
            _ => Outcome::Continue,
        };

        self.popup.active.set_lost(false);
        self.popup.active.set_gained(false);

        r
    }
}

impl<T: PartialEq> HandleEvent<crossterm::event::Event, Popup, Outcome> for ChoiceState<T> {
    fn handle(&mut self, event: &crossterm::event::Event, _qualifier: Popup) -> Outcome {
        let r1 = match self.popup.handle(event, Popup) {
            PopupOutcome::Hide => {
                self.set_popup_active(false);
                Outcome::Changed
            }
            r => r.into(),
        };

        let mut sas = ScrollAreaState::new()
            .area(self.popup.area)
            .v_scroll(&mut self.popup.v_scroll);
        let mut r2 = match sas.handle(event, MouseOnly) {
            ScrollOutcome::Up(n) => self.move_up(n).into(),
            ScrollOutcome::Down(n) => self.move_down(n).into(),
            ScrollOutcome::VPos(n) => self.move_to(n).into(),
            _ => Outcome::Continue,
        };

        r2 = r2.or_else(|| match event {
            ct_event!(mouse any for m) if self.mouse.doubleclick(self.popup.widget_area, m) => {
                if let Some(n) = item_at(&self.item_areas, m.column, m.row) {
                    let r = self.move_to(self.offset() + n).into();
                    let s = self.set_popup_active(false).into();
                    max(r, s)
                } else {
                    Outcome::Unchanged
                }
            }
            ct_event!(mouse down Left for x,y)
                if self.popup.widget_area.contains((*x, *y).into()) =>
            {
                if let Some(n) = item_at(&self.item_areas, *x, *y) {
                    self.move_to(self.offset() + n).into()
                } else {
                    Outcome::Unchanged
                }
            }
            ct_event!(mouse drag Left for x,y)
                if self.popup.widget_area.contains((*x, *y).into()) =>
            {
                if let Some(n) = item_at(&self.item_areas, *x, *y) {
                    self.move_to(self.offset() + n).into()
                } else {
                    Outcome::Unchanged
                }
            }
            _ => Outcome::Continue,
        });

        r2 = r2.or_else(|| mouse_trap(event, self.popup.area));

        max(r1, r2)
    }
}

/// Handle events for the popup.
/// Call before other handlers to deal with intersections
/// with other widgets.
pub fn handle_popup<T: PartialEq>(
    state: &mut ChoiceState<T>,
    focus: bool,
    event: &crossterm::event::Event,
) -> Outcome {
    state.focus.set(focus);
    HandleEvent::handle(state, event, Popup)
}

/// Handle all events.
/// Text events are only processed if focus is true.
/// Mouse events are processed if they are in range.
pub fn handle_events<T: PartialEq>(
    state: &mut ChoiceState<T>,
    focus: bool,
    event: &crossterm::event::Event,
) -> Outcome {
    state.focus.set(focus);
    HandleEvent::handle(state, event, Regular)
}

/// Handle only mouse-events.
pub fn handle_mouse_events<T: PartialEq>(
    state: &mut ChoiceState<T>,
    event: &crossterm::event::Event,
) -> Outcome {
    HandleEvent::handle(state, event, MouseOnly)
}
