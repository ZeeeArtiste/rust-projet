use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
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

        // Ajout de quelques ressources pour le test.
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

    fn print(&self) {
        for row in &self.data {
            let line: String = row.iter().collect();
            println!("{}", line);
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
}

impl Robot {
    fn new(id: usize, x: usize, y: usize, robot_type: RobotType) -> Self {
        Self {
            id,
            x,
            y,
            robot_type,
            inventory: 0,
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

    fn perform_task(&mut self, map: &mut Map, logs: &Arc<Mutex<Vec<String>>>) {
        match self.robot_type {
            RobotType::Explorer => {
                let tile = map.data[self.y][self.x];
                if tile == 'M' || tile == 'E' {
                    log_event(
                        logs,
                        &format!(
                            "Explorer {} a trouvé une ressource en ({}, {})",
                            self.id, self.x, self.y
                        ),
                    );
                }
                self.move_randomly(map.width, map.height, map);
            }
            RobotType::Miner => {
                if self.inventory < MAX_INVENTORY {
                    self.move_randomly(map.width, map.height, map);
                } else {
                    self.move_towards((map.base_x, map.base_y), map);
                    if self.x == map.base_x && self.y == map.base_y {
                        log_event(logs, &format!("Miner {} se vide à la base", self.id));
                        self.inventory = 0;
                    }
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let map = Arc::new(Mutex::new(Map::new(80, 30, 42)));
    let logs = Arc::new(Mutex::new(Vec::new()));
    let robots = Arc::new(Mutex::new(vec![
        Robot::new(
            0,
            map.lock().unwrap().base_x,
            map.lock().unwrap().base_y,
            RobotType::Explorer,
        ),
        Robot::new(
            1,
            map.lock().unwrap().base_x,
            map.lock().unwrap().base_y,
            RobotType::Miner,
        ),
    ]));
    let (tx, rx) = mpsc::channel();

    // Démarrage d'un thread par robot
    for i in 0..2 {
        let map_clone = Arc::clone(&map);
        let robots_clone = Arc::clone(&robots);
        let tx_clone = tx.clone();
        let logs_clone = Arc::clone(&logs);
        let running_clone = Arc::clone(&running);
        thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                {
                    let mut map_lock = map_clone.lock().unwrap();
                    let mut robots_lock = robots_clone.lock().unwrap();
                    robots_lock[i].perform_task(&mut map_lock, &logs_clone);
                    let _ = tx_clone.send(map_lock.clone());
                }
                thread::sleep(Duration::from_millis(200));
            }
        });
    }

    for _ in 0..10 {
        if let Ok(updated_map) = rx.recv_timeout(Duration::from_millis(500)) {
            println!("\n=== Mise à jour de la carte ===");
            updated_map.print();
        }
    }

    running.store(false, Ordering::SeqCst);
    Ok(())
}
