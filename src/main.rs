use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Styled},
    symbols,
    widgets::{
      Axis, Block, Chart, Dataset, Gauge, GraphType
    }
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    system: sysinfo::System,
    disks: sysinfo::Disks,
    cpu: Vec<(f64, f64)>,
    memory: Vec<(f64, f64)>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
          running: true,
          system: sysinfo::System::new_all(),
          disks: sysinfo::Disks::new_with_refreshed_list(),
          cpu: vec![],
          memory: vec![],
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            self.system.refresh_cpu_all();
            self.disks.refresh(false);
            terminal.draw(|frame| {
              self.cpu.push((frame.count() as f64, self.system.global_cpu_usage() as f64));
              let mem_pct = self.system.used_memory() as f64 / self.system.total_memory() as f64 * 100.0;
              self.memory.push((frame.count() as f64, mem_pct));
              self.render(frame)
            })?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, _frame: &mut Frame) {
        let [first, second] = Layout::vertical([
          Constraint::Percentage(40),
          Constraint::Percentage(60)
        ]).areas(_frame.area());
        let datasets = vec![
          // Scatter chart
          Dataset::default()
              .name("CPU")
              .marker(symbols::Marker::Braille)
              .graph_type(GraphType::Line)
              .style(Style::default().red())
              .data(&self.cpu),
          Dataset::default()
              .name("Memory")
              .marker(symbols::Marker::Braille)
              .graph_type(GraphType::Line)
              .style(Style::default().blue())
              .data(&self.memory),
        ];

        let x_axis = Axis::default()
            .bounds([0f64, self.cpu.len() as f64])
            .title("Time");
        let y_axis = Axis::default()
            .bounds([0f64, 100f64])
            .labels(["0%", "25%", "50%", "75%", "100%"]);

        let chart = Chart::new(datasets)
            .block(Block::bordered().title("CPU & Memory Usage (%)"))
            .x_axis(x_axis)
            .y_axis(y_axis);

        let disk_data: Vec<(String, f64)> = self.disks.list().iter().map(|d| {
            let used = d.total_space().saturating_sub(d.available_space());
            let ratio = if d.total_space() > 0 { used as f64 / d.total_space() as f64 } else { 0.0 };
            let label = format!("{} — {:.1} / {:.1} GB",
                d.mount_point().display(),
                used as f64 / 1e9,
                d.total_space() as f64 / 1e9,
            );
            (label, ratio)
        }).collect();

        let disk_block = Block::bordered().title("Disk Usage");
        let inner = disk_block.inner(second);
        _frame.render_widget(disk_block, second);

        let row_constraints: Vec<Constraint> = disk_data.iter()
            .map(|_| Constraint::Length(3))
            .collect();
        let rows = Layout::vertical(row_constraints).split(inner);
        for (i, (label, ratio)) in disk_data.iter().enumerate() {
            if i >= rows.len() { break; }
            let gauge = Gauge::default()
                .block(Block::bordered())
                .gqauge_style(Style::default().fg(Color::Red).bg(Color::White))
                .label(label.as_str())
                .ratio(*ratio);
            _frame.render_widget(gauge, rows[i]);
        }

        _frame.render_widget(chart, first);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        if event::poll(std::time::Duration::from_millis(30))? {
          match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
          }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
