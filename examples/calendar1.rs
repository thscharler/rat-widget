#![allow(dead_code)]

use crate::mini_salsa::theme::THEME;
use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use chrono::NaiveDate;
use pure_rust_locales::Locale;
use rat_event::{try_flow, ConsumedEvent, HandleEvent, Regular};
use rat_focus::{Focus, FocusBuilder};
use rat_menu::event::MenuOutcome;
use rat_menu::menuline::{MenuLine, MenuLineState};
use rat_widget::calendar::{Month, MonthState};
use rat_widget::event::Outcome;
use rat_widget::statusline::StatusLineState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::Frame;
use std::collections::HashMap;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        cal: Default::default(),
        menu: Default::default(),
        status: Default::default(),
    };
    state.menu.focus.set(true);

    run_ui(
        "calendar1",
        handle_input,
        repaint_input,
        &mut data,
        &mut state,
    )
}

struct Data {}

struct State {
    pub(crate) cal: MonthState,
    pub(crate) menu: MenuLineState,
    pub(crate) status: StatusLineState,
}

fn repaint_input(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let l1 = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    let l2 = Layout::horizontal([
        Constraint::Length(25),
        Constraint::Fill(1),
        Constraint::Length(15),
    ])
    .split(l1[1]);

    let mut date_styles = HashMap::new();
    date_styles.insert(
        NaiveDate::from_ymd_opt(2024, 9, 1).expect("some"),
        Style::default().red(),
    );

    let cal = Month::new()
        .date(chrono::offset::Local::now().date_naive())
        .locale(Locale::de_AT_euro)
        .styles(THEME.month_style())
        .day_styles(&date_styles)
        .day_selection()
        .week_selection()
        .show_weekdays()
        .block(Block::bordered());
    cal.render(l2[1], frame.buffer_mut(), &mut state.cal);

    let menu1 = MenuLine::new()
        .title("||||")
        .item_parsed("_Quit")
        .title_style(Style::default().black().on_yellow())
        .style(Style::default().black().on_dark_gray());
    frame.render_stateful_widget(menu1, l1[3], &mut state.menu);

    Ok(())
}

fn focus(state: &State) -> Focus {
    let mut fb = FocusBuilder::default();
    fb.widget(&state.cal).widget(&state.menu);
    fb.build()
}

fn handle_input(
    event: &crossterm::event::Event,
    _data: &mut Data,
    istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    let f = focus(state).handle(event, Regular);
    let r = f.and_try(|| {
        try_flow!({
            let r = HandleEvent::handle(&mut state.cal, event, Regular);
            r
        });

        try_flow!(match state.menu.handle(event, Regular) {
            MenuOutcome::Activated(0) => {
                istate.quit = true;
                Outcome::Changed
            }
            _ => {
                Outcome::Continue
            }
        });
        Ok(Outcome::Continue)
    });
    r
}
