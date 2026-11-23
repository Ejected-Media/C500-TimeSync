use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use mud_core::{
    storage::{HybridStorage, PlayerSave},
    GameSession, Level, Room,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    time::Duration,
};
use timesync_bridge::{TimeSyncSnapshot, WorldModifiers};
use tokio::time::interval;

// Antistatic Lab High-Contrast Palette (Avoids dark-on-dark low contrast)
const BG_SLATE: Color = Color::Rgb(20, 23, 22);
const FG_PLATINUM: Color = Color::Rgb(232, 235, 233);
const ACCENT_PHOSPHOR: Color = Color::Rgb(74, 246, 38);
const ACCENT_AMBER: Color = Color::Rgb(255, 176, 0);
const ACCENT_ALERT: Color = Color::Rgb(255, 75, 75);
const BORDER_DIM: Color = Color::Rgb(110, 122, 118);

struct App {
    session: GameSession,
    sync_snapshot: TimeSyncSnapshot,
    modifiers: WorldModifiers,
    endpoint_url: String,
    input: String,
    log_lines: Vec<String>,
    should_quit: bool,
}

impl App {
    fn new(session: GameSession, endpoint_url: String) -> Self {
        let default_snapshot = TimeSyncSnapshot {
            active_session_id: None,
            local_time_12h: "10:00 AM".into(),
            iana_timezone: "America/Phoenix".into(),
            is_daylight: true,
            weather_code: 0,
            emergency_lockdown: false,
        };
        let modifiers = default_snapshot.to_world_modifiers();

        let current_room = &session.current_level.rooms[&session.player.current_room_id];
        let initial_logs = vec![
            format!("=== EJECTED MEDIA // C500-TIMESYNC MUD ==="),
            format!("Level {}: {}", session.current_level.id, session.current_level.title),
            format!("[{}]", current_room.name),
            current_room.description.clone(),
            "Tip: Use n/s/e/w to move, 'sync' to poll TimeSync, 'logs' for fjall history.".into(),
        ];

        Self {
            session,
            sync_snapshot: default_snapshot,
            modifiers,
            endpoint_url,
            input: String::new(),
            log_lines: initial_logs,
            should_quit: false,
        }
    }

    async fn refresh_timesync(&mut self) {
        match TimeSyncSnapshot::fetch(&self.endpoint_url).await {
            Ok(snap) => {
                self.modifiers = snap.to_world_modifiers();
                self.sync_snapshot = snap;
                self.log_lines.push(format!(
                    "[SYNC OK] {} ({}) | Portal: {}",
                    self.sync_snapshot.local_time_12h,
                    self.sync_snapshot.iana_timezone,
                    if self.modifiers.portal_open { "OPEN" } else { "STANDBY" }
                ));
            }
            Err(err) => {
                self.log_lines.push(format!("[SYNC OFFLINE] Using cached state ({})", err));
            }
        }
    }

    async fn execute_command(&mut self, raw_cmd: &str) {
        let cmd = raw_cmd.trim().to_lowercase();
        if cmd.is_empty() {
            return;
        }

        self.log_lines.push(format!("> {}", raw_cmd.trim()));

        // Normalize mobile single-letter shortcuts
        let normalized = match cmd.as_str() {
            "n" | "north" => "move north",
            "s" | "south" => "move south",
            "e" | "east" => "move east",
            "w" | "west" => "move west",
            "l" => "look",
            other => other,
        };

        if normalized == "quit" || normalized == "exit" {
            self.should_quit = true;
            return;
        }

        if normalized == "sync" {
            self.refresh_timesync().await;
            return;
        }

        if normalized == "look" {
            if let Some(room) = self.session.current_level.rooms.get(&self.session.player.current_room_id) {
                self.log_lines.push(format!("[{}]", room.name));
                self.log_lines.push(room.description.clone());
            }
            return;
        }

        if normalized == "logs" {
            match self.session.storage.recent_events(&self.session.player.player_id, 5) {
                Ok(events) => {
                    self.log_lines.push("--- Recent fjall Event Log ---".into());
                    for ev in events {
                        self.log_lines.push(format!("[{}] {}: {}", ev.timestamp_unix, ev.event_type, ev.payload));
                    }
                }
                Err(e) => self.log_lines.push(format!("Log read error: {}", e)),
            }
            return;
        }

        if let Some(dir) = normalized.strip_prefix("move ") {
            if self.sync_snapshot.emergency_lockdown {
                self.log_lines.push("BLOCKED: C500-TimeSync Emergency Cancel lockdown is active.".into());
                return;
            }
            match self.session.move_player(dir.trim()) {
                Ok(output) => {
                    for line in output.lines() {
                        if !line.is_empty() {
                            self.log_lines.push(line.to_string());
                        }
                    }
                }
                Err(e) => self.log_lines.push(format!("Cannot move: {}", e)),
            }
            return;
        }

        self.log_lines.push(format!("Unknown command '{}'. Try: n, s, e, w, look, sync, logs, quit", cmd));
    }
}

fn termux_data_dir() -> PathBuf {
    if let Ok(prefix) = std::env::var("PREFIX") {
        PathBuf::from(prefix).join("var/lib/ejected-mud")
    } else {
        PathBuf::from("./.ejected-mud-data")
    }
}

fn bootstrap_demo_level() -> Level {
    let mut rooms = HashMap::new();

    let mut hub_exits = HashMap::new();
    hub_exits.insert("north".into(), "sync_gate".into());
    hub_exits.insert("east".into(), "lab_bench".into());

    rooms.insert(
        "c500_hub".into(),
        Room {
            name: "C500 Dispatch Atrium".into(),
            description: "Exposed conduit and platinum CRT terminals hum softly. The north archway leads toward the TimeSync Portal.".into(),
            exits: hub_exits,
            items: vec!["Keycap Puller".into()],
            sync_event_hook: None,
        },
    );

    let mut gate_exits = HashMap::new();
    gate_exits.insert("south".into(), "c500_hub".into());

    rooms.insert(
        "sync_gate".into(),
        Room {
            name: "MWF 10:00 AM Chronos Gate".into(),
            description: "An amber laser grid pulses in time with America/Phoenix IANA ticks, waiting for an active /book-session window.".into(),
            exits: gate_exits,
            items: vec![],
            sync_event_hook: Some("CHECK_ACTIVE_SESSION".into()),
        },
    );

    let mut lab_exits = HashMap::new();
    lab_exits.insert("west".into(), "c500_hub".into());

    rooms.insert(
        "lab_bench".into(),
        Room {
            name: "Antistatic Soldering Bay".into(),
            description: "RP2040 controller PCBs and anodized keycap trays rest under ESD-safe task lamps.".into(),
            exits: lab_exits,
            items: vec!["RP2040 DevBoard".into()],
            sync_event_hook: None,
        },
    );

    Level {
        id: 1,
        title: "Classroom 500 Boot Sector".into(),
        required_sync_state: None,
        starting_room: "c500_hub".into(),
        rooms,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize redb + fjall storage in Termux $PREFIX/var/lib/ejected-mud
    let storage = HybridStorage::open(termux_data_dir())?;
    let level = bootstrap_demo_level();

    let player_id = "kyle2501";
    let player = match storage.load_player(player_id)? {
        Some(saved) => saved,
        None => {
            let fresh = PlayerSave {
                player_id: player_id.into(),
                active_level_id: level.id,
                current_room_id: level.starting_room.clone(),
                inventory: vec![],
                xp: 0,
                last_synced_session: None,
            };
            storage.save_player(&fresh)?;
            fresh
        }
    };

    let session = GameSession {
        storage,
        player,
        current_level: level,
    };

    let endpoint = std::env::var("C500_TIMESYNC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".into());

    let mut app = App::new(session, endpoint);
    app.refresh_timesync().await;

    // 2. Setup Crossterm alternate screen
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut reader = EventStream::new();
    let mut sync_ticker = interval(Duration::from_secs(60));

    // 3. Main async event loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            _ = sync_ticker.tick() => {
                app.refresh_timesync().await;
            }
            Some(Ok(event)) = reader.next() => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                let cmd = std::mem::take(&mut app.input);
                                app.execute_command(&cmd).await;
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // 4. Clean terminal restore
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    // Vertical portrait layout optimized for Termux + Android soft keyboard
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top C500-TimeSync status header
            Constraint::Min(8),    // Scrollable MUD narrative feed
            Constraint::Length(3), // Exits & player HUD strip
            Constraint::Length(3), // Command input prompt
        ])
        .split(area);

    // 1. Top Status Bar
    let portal_badge = if app.sync_snapshot.emergency_lockdown {
        Span::styled(" LOCKDOWN ", Style::default().fg(BG_SLATE).bg(ACCENT_ALERT).add_modifier(Modifier::BOLD))
    } else if app.modifiers.portal_open {
        Span::styled(
            format!(" PORTAL OPEN ({:.1}x XP) ", app.modifiers.xp_multiplier),
            Style::default().fg(BG_SLATE).bg(ACCENT_PHOSPHOR).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" STANDBY ", Style::default().fg(BG_SLATE).bg(ACCENT_AMBER).add_modifier(Modifier::BOLD))
    };

    let header_line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.sync_snapshot.local_time_12h),
            Style::default().fg(ACCENT_PHOSPHOR).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", app.sync_snapshot.iana_timezone),
            Style::default().fg(FG_PLATINUM),
        ),
        portal_badge,
    ]);

    let header = Paragraph::new(header_line)
        .style(Style::default().bg(BG_SLATE).fg(FG_PLATINUM))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .title(" C500-TimeSync "),
        );
    f.render_widget(header, chunks[0]);

    // 2. Main Narrative Log (Auto-scrolled to bottom lines)
    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    let skip = app.log_lines.len().saturating_sub(inner_height);
    let visible_lines: Vec<Line> = app.log_lines[skip..]
        .iter()
        .map(|l| {
            let style = if l.starts_with('>') {
                Style::default().fg(ACCENT_AMBER).add_modifier(Modifier::BOLD)
            } else if l.starts_with('[') {
                Style::default().fg(ACCENT_PHOSPHOR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG_PLATINUM)
            };
            Line::from(Span::styled(l.as_str(), style))
        })
        .collect();

    let feed = Paragraph::new(visible_lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(BG_SLATE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .title(format!(" LVL {} ", app.session.player.active_level_id)),
        );
    f.render_widget(feed, chunks[1]);

    // 3. Exits & HUD Strip
    let exits_text = app
        .session
        .current_level
        .rooms
        .get(&app.session.player.current_room_id)
        .map(|r| {
            let mut keys: Vec<String> = r.exits.keys().map(|k| format!("[{}]", k.to_uppercase())).collect();
            keys.sort();
            if keys.is_empty() { "NONE".into() } else { keys.join(" ") }
        })
        .unwrap_or_else(|| "NONE".into());

    let hud_line = Line::from(vec![
        Span::styled(" EXITS: ", Style::default().fg(BORDER_DIM)),
        Span::styled(exits_text, Style::default().fg(ACCENT_PHOSPHOR).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  |  XP: {}", app.session.player.xp), Style::default().fg(FG_PLATINUM)),
    ]);

    let hud = Paragraph::new(hud_line)
        .style(Style::default().bg(BG_SLATE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM)),
        );
    f.render_widget(hud, chunks[2]);

    // 4. Command Input Prompt
    let input_box = Paragraph::new(format!("> {}", app.input))
        .style(Style::default().bg(BG_SLATE).fg(ACCENT_AMBER))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT_PHOSPHOR))
                .title(" CMD "),
        );
    f.render_widget(input_box, chunks[3]);
}
