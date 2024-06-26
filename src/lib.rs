#![doc = include_str!("../readme.md")]
#![allow(clippy::collapsible_else_if)]

pub mod button;
pub mod calender;
pub mod date_input;
pub mod edit_table;
pub mod input;
pub mod list;
pub mod masked_input;
pub mod menubar;
pub mod menuline;
pub mod number_input;
pub mod popup_menu;
pub mod table;
pub mod textarea;

pub use pure_rust_locales::Locale;

/// Module for focus-handling functionality.
/// For details see [rat-focus](https://docs.rs/rat-focus)
pub mod focus {
    pub use rat_focus::{
        match_focus, on_gained, on_lost, Focus, FocusFlag, HasFocus, HasFocusFlag, ZRect,
    };
}

/// Scrolled widget and viewports.
pub mod scrolled {
    pub use rat_scrolled::{
        HScrollPosition, Inner, ScrollbarPolicy, Scrolled, ScrolledState, ScrolledStyle,
        ScrollingState, ScrollingWidget, VScrollPosition, View, ViewState, Viewport, ViewportState,
    };
}

/// Event-handling traits and types.
pub mod event {
    pub use rat_ftable::event::{DoubleClick, DoubleClickOutcome, EditKeys, EditOutcome};
    pub use rat_input::event::{
        crossterm, ct_event, flow, flow_ok, util, ConsumedEvent, FocusKeys, HandleEvent, MouseOnly,
        Outcome, Popup, ReadOnly, TextOutcome,
    };
    pub use rat_scrolled::event::ScrollOutcome;
}

/// Layout calculation.
pub mod layout {
    pub use rat_input::layout_dialog::{layout_dialog, LayoutDialog};
    pub use rat_input::layout_edit::{layout_edit, EditConstraint, LayoutEdit, LayoutEditIterator};
    pub use rat_input::layout_grid::layout_grid;
}

/// Basic message dialog.
pub mod msgdialog {
    pub use rat_input::msgdialog::{MsgDialog, MsgDialogState, MsgDialogStyle};
}

/// Statusbar.
pub mod statusline {
    pub use rat_input::statusline::{StatusLine, StatusLineState};
}

/// Fill an area with a Style and a symbol.
pub mod fill {
    pub use rat_input::fill::Fill;
}

mod _private {
    // todo: remvoe
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
