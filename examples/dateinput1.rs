use crate::mini_salsa::{run_ui, setup_logging, MiniSalsaState};
use rat_event::try_flow;
use rat_text::HasScreenCursor;
use rat_widget::date_input;
use rat_widget::date_input::{DateInput, DateInputState};
use rat_widget::event::Outcome;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::Frame;

mod mini_salsa;

fn main() -> Result<(), anyhow::Error> {
    setup_logging()?;

    let mut data = Data {};

    let mut state = State {
        input: DateInputState::new().with_pattern("%x")?,
    };

    run_ui(
        "dateinput1",
        handle_input,
        repaint_input,
        &mut data,
        &mut state,
    )
}

struct Data {}

struct State {
    pub(crate) input: DateInputState,
}

fn repaint_input(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<(), anyhow::Error> {
    let l0 = Layout::horizontal([
        Constraint::Length(25),
        Constraint::Length(20),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .split(area);

    let l1 = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(l0[1]);

    let l2 = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(l0[1]);

    let input1 = DateInput::default().style(Style::default().white().on_dark_gray());
    frame.render_stateful_widget(input1, l1[1], &mut state.input);
    if let Some((x, y)) = state.input.screen_cursor() {
        frame.set_cursor_position((x, y));
    }

    let txt1 = Span::from(format!("{:?}", state.input.value()));
    frame.render_widget(txt1, l2[1]);

    Ok(())
}

fn handle_input(
    event: &crossterm::event::Event,
    _data: &mut Data,
    _istate: &mut MiniSalsaState,
    state: &mut State,
) -> Result<Outcome, anyhow::Error> {
    try_flow!(date_input::handle_events(&mut state.input, true, event));
    Ok(Outcome::Continue)
}
