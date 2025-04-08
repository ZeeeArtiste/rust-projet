use noise::{NoiseFn, Perlin};
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    prelude::*,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::*,
};
use std::collections::HashSet;
use std::io::{self, stdout};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

const MAX_LOGS: usize = 10;
const MAX_INVENTORY: u32 = 5;

fn log_event(logs: &Arc<Mutex<Vec<String>>>, msg: &str) {
    let mut logs = logs.lock().unwrap();
    logs.push(msg.to_string());
    if logs.len() > MAX_LOGS {
        logs.remove(0);
    }
}

#[derive(Debug, Clone)]
struct Map {
    width: usize,
    height: usize,
    data: Vec<Vec<char>>,
    base_x: usize,
    base_y: usize,
}

impl Map {
    fn new(width: usize, height: usize, seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let mut data = vec![vec!['.'; width]; height];

        for y in 0..height {
            for x in 0..width {
                let noise_value = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                if noise_value > 0.4 {
                    data[y][x] = '#';
                }
            }
        }

        let base_x = width / 2;
        let base_y = height / 2;
        data[base_y][base_x] = 'S';

        let mut rng = rand::thread_rng();
        for _ in 0..10 {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            if data[y][x] == '.' {
                data[y][x] = if rng.gen_bool(0.5) { 'M' } else { 'E' };
            }
        }

        Self {
            width,
            height,
            data,
            base_x,
            base_y,
        }
    }

    fn clone_map(&self) -> Map {
        Self {
            width: self.width,
            height: self.height,
            data: self.data.clone(),
            base_x: self.base_x,
            base_y: self.base_y,
        }
    }
}

#[derive(Debug, Clone)]
enum RobotType {
    Explorer,
    Miner,
}

#[derive(Debug, Clone)]
struct Robot {
    id: usize,
    x: usize,
    y: usize,
    robot_type: RobotType,
    inventory: u32,
    target: Option<(usize, usize)>,
    paused: bool,
}

impl Robot {
    fn new(id: usize, x: usize, y: usize, robot_type: RobotType) -> Self {
        Self {
            id,
            x,
            y,
            robot_type,
            inventory: 0,
            target: None,
            paused: false,
        }
    }

    fn move_randomly(&mut self, width: usize, height: usize, map: &Map) {
        let mut rng = rand::thread_rng();
        let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        let (dx, dy) = directions[rng.gen_range(0..directions.len())];
        let new_x = ((self.x as isize + dx) + width as isize) % width as isize;
        let new_y = ((self.y as isize + dy) + height as isize) % height as isize;
        if map.data[new_y as usize][new_x as usize] != '#' {
            self.x = new_x as usize;
            self.y = new_y as usize;
        }
    }

    fn move_towards(&mut self, target: (usize, usize), map: &Map) {
        let (target_x, target_y) = target;
        let mut new_x = self.x;
        let mut new_y = self.y;
        if self.x < target_x {
            new_x += 1;
        } else if self.x > target_x {
            new_x -= 1;
        }
        if self.y < target_y {
            new_y += 1;
        } else if self.y > target_y {
            new_y -= 1;
        }
        if map.data[new_y][new_x] != '#' {
            self.x = new_x;
            self.y = new_y;
        } else {
            self.move_randomly(map.width, map.height, map);
        }
    }

    fn perform_task(
        &mut self,
        map: &mut Map,
        reported_resources: &Arc<Mutex<HashSet<(usize, usize)>>>,
        logs: &Arc<Mutex<Vec<String>>>,
    ) {
        match self.robot_type {
            RobotType::Explorer => {
                let tile = map.data[self.y][self.x];
                if tile == 'M' || tile == 'E' {
                    let mut rep = reported_resources.lock().unwrap();
                    rep.insert((self.x, self.y));
                    log_event(
                        logs,
                        &format!(
                            "Explorer a trouvé une ressource en ({}, {})",
                            self.x, self.y
                        ),
                    );
                }
                self.move_randomly(map.width, map.height, map);
            }
            RobotType::Miner => {
                if self.inventory < MAX_INVENTORY {
                    if self.id == 2 {
                        let rep = reported_resources.lock().unwrap();
                        if rep.len() < 2 && self.target.is_none() {
                            if self.x == map.base_x && self.y == map.base_y {
                                if map.base_x + 1 < map.width {
                                    self.x = map.base_x + 1;
                                } else if map.base_x > 0 {
                                    self.x = map.base_x - 1;
                                }
                            }
                            if !self.paused {
                                log_event(logs, "Robot 2 en pause (attente de 2 ressources)");
                                self.paused = true;
                            }
                            return;
                        } else {
                            self.paused = false;
                        }
                    }
                    if self.target.is_none() {
                        let rep = reported_resources.lock().unwrap();
                        if self.id == 2 {
                            if rep.len() >= 2 {
                                if let Some(&target) = rep.iter().nth(1) {
                                    self.target = Some(target);
                                    log_event(
                                        logs,
                                        &format!(
                                            "Robot 2 se mobilise sur la ressource en ({}, {})",
                                            target.0, target.1
                                        ),
                                    );
                                }
                            }
                        } else {
                            if let Some(&target) = rep.iter().next() {
                                self.target = Some(target);
                                log_event(
                                    logs,
                                    &format!(
                                        "Robot 1 se mobilise sur la ressource en ({}, {})",
                                        target.0, target.1
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(target) = self.target {
                        self.move_towards(target, map);
                        if self.x == target.0 && self.y == target.1 {
                            if map.data[self.y][self.x] == 'M' || map.data[self.y][self.x] == 'E' {
                                map.data[self.y][self.x] = '.';
                                self.inventory += 1;
                                log_event(
                                    logs,
                                    &format!(
                                        "Ressource collectée par robot {} (inventaire: {})",
                                        self.id, self.inventory
                                    ),
                                );
                            }
                            let mut rep = reported_resources.lock().unwrap();
                            rep.remove(&target);
                            self.target = None;
                        }
                    } else {
                        self.move_randomly(map.width, map.height, map);
                    }
                } else {
                    self.move_towards((map.base_x, map.base_y), map);
                    if self.x == map.base_x && self.y == map.base_y {
                        log_event(
                            logs,
                            &format!(
                                "Robot {} vient de se vider (inventaire: {})",
                                self.id, self.inventory
                            ),
                        );
                        self.inventory = 0;
                    }
                }
            }
        }
    }
}

fn render_ui(
    rx: mpsc::Receiver<Map>,
    robots: Arc<Mutex<Vec<Robot>>>,
    running: Arc<AtomicBool>,
    logs: Arc<Mutex<Vec<String>>>,
) -> io::Result<()> {
    let stdout = stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    while running.load(Ordering::SeqCst) {
        if let Ok(map) = rx.recv_timeout(Duration::from_millis(100)) {
            let robots_guard = robots.lock().unwrap();
            let mut sim_lines: Vec<Line> = Vec::with_capacity(map.height);
            for y in 0..map.height {
                let mut spans: Vec<Span> = Vec::with_capacity(map.width);
                for x in 0..map.width {
                    let mut ch = map.data[y][x];
                    let mut style = match ch {
                        '#' => Style::default().fg(Color::DarkGray),
                        'S' => Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                        'M' | 'E' => Style::default().fg(Color::Yellow),
                        '.' => Style::default().fg(Color::White),
                        _ => Style::default(),
                    };
                    for robot in robots_guard.iter() {
                        if robot.x == x && robot.y == y {
                            ch = match robot.robot_type {
                                RobotType::Explorer => 'X',
                                RobotType::Miner => 'R',
                            };
                            style = match robot.robot_type {
                                RobotType::Explorer => Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                                RobotType::Miner => Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            };
                            break;
                        }
                    }
                    spans.push(Span::styled(ch.to_string(), style));
                }
                sim_lines.push(Line::from(spans));
            }

            let log_lines: Vec<Line> = {
                let logs_lock = logs.lock().unwrap();
                logs_lock
                    .iter()
                    .map(|l| Line::from(Span::raw(l.clone())))
                    .collect()
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(terminal.size()?);

            let sim_paragraph = Paragraph::new(sim_lines)
                .block(Block::default().borders(Borders::ALL).title("Simulation"));
            let log_paragraph = Paragraph::new(log_lines)
                .block(Block::default().borders(Borders::ALL).title("Logs"));

            terminal.draw(|frame| {
                frame.render_widget(sim_paragraph, chunks[0]);
                frame.render_widget(log_paragraph, chunks[1]);
            })?;
        }
    }
    terminal.clear()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Erreur lors de la configuration du handler Ctrl-C");

    // Map
    let initial_map = Map::new(150, 50, 42);
    let base_x = initial_map.base_x;
    let base_y = initial_map.base_y;
    let map = Arc::new(Mutex::new(initial_map));

    let robots = Arc::new(Mutex::new(vec![
        Robot::new(0, base_x, base_y, RobotType::Explorer),
        Robot::new(1, base_x, base_y, RobotType::Miner),
        Robot::new(2, base_x, base_y, RobotType::Miner),
    ]));

    let reported_resources = Arc::new(Mutex::new(HashSet::new()));
    let logs = Arc::new(Mutex::new(Vec::new()));

    let (tx, rx) = mpsc::channel();

    // Robots
    for i in 0..3 {
        let map_shared = Arc::clone(&map);
        let tx_clone = tx.clone();
        let robots_shared = Arc::clone(&robots);
        let running_clone = Arc::clone(&running);
        let reported_resources_clone = Arc::clone(&reported_resources);
        let logs_clone = Arc::clone(&logs);

        thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                {
                    let mut map = map_shared.lock().unwrap();
                    let mut robots = robots_shared.lock().unwrap();
                    let mut robot = robots[i].clone();
                    robot.perform_task(&mut map, &reported_resources_clone, &logs_clone);
                    robots[i] = robot.clone();
                    let _ = tx_clone.send(map.clone_map());
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    // UI
    let robots_ui = Arc::clone(&robots);
    let running_ui = Arc::clone(&running);
    let logs_ui = Arc::clone(&logs);
    let ui_handle = thread::spawn(move || {
        if let Err(e) = render_ui(rx, robots_ui, running_ui, logs_ui) {
            eprintln!("Erreur dans l'UI : {}", e);
        }
    });

    ui_handle.join().unwrap();
    Ok(())
}
