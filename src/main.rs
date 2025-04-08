use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::io;

const MAX_LOGS: usize = 10;

#[derive(Debug)]
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

        // Génération d'un terrain simple grâce au bruit de Perlin
        for y in 0..height {
            for x in 0..width {
                let noise_value = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                if noise_value > 0.4 {
                    data[y][x] = '#';
                }
            }
        }

        // Position de la base au centre de la carte
        let base_x = width / 2;
        let base_y = height / 2;
        data[base_y][base_x] = 'S';

        Self {
            width,
            height,
            data,
            base_x,
            base_y,
        }
    }
}

fn main() -> io::Result<()> {
    // Création d'une carte de test.
    let map = Map::new(80, 30, 42);
    println!("Carte générée : {} colonnes x {} lignes", map.width, map.height);
    println!("Base positionnée en ({}, {})", map.base_x, map.base_y);
    Ok(())
}
