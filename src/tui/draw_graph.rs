use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use std::f32::consts::PI;
use std::io::stdout;
use std::{thread, time::Duration};

fn clear_screen() {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
}

pub fn draw_ascii_graph(adj: &[Vec<u8>]) {
    let n = adj.len();
    if n == 0 {
        println!("Empty graph.");
        return;
    }

    let width = 40;
    let height = 20;
    let mut canvas = vec![vec![' '; width]; height];
    let radius = (height.min(width) / 2 - 2) as f32;

    // Calculate node positions in a circle
    let mut positions = Vec::new();
    for i in 0..n {
        let angle = 2.0 * PI * (i as f32) / (n as f32);
        let x = (width as f32 / 2.0 + radius * angle.cos()).round() as usize;
        let y = (height as f32 / 2.0 + radius * angle.sin()).round() as usize;
        positions.push((x, y));
    }

    // Draw edges
    for i in 0..n {
        for j in 0..n {
            if i < j && adj[i][j] == 1 {
                draw_line(&mut canvas, positions[i], positions[j]);
            }
        }
    }

    // Draw nodes
    for (i, &(x, y)) in positions.iter().enumerate() {
        let label = format!("{}", i);
        let chars: Vec<char> = label.chars().collect();
        if y < height && x < width {
            canvas[y][x] = '(';
            if x + 1 < width {
                canvas[y][x + 1] = chars[0];
            }
            if x + 2 < width {
                canvas[y][x + 2] = ')';
            }
        }
    }

    // Print canvas
    for row in canvas {
        println!("{}", row.iter().collect::<String>());
    }
}

// Bresenham-style line drawing
fn draw_line(canvas: &mut Vec<Vec<char>>, from: (usize, usize), to: (usize, usize)) {
    let (x0, y0) = (from.0 as isize, from.1 as isize);
    let (x1, y1) = (to.0 as isize, to.1 as isize);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < canvas[0].len() as isize && y >= 0 && y < canvas.len() as isize {
            canvas[y as usize][x as usize] = '*';
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn draw() {
    let graphs = vec![
        vec![
            vec![0, 1, 1, 0, 0, 0],
            vec![1, 0, 0, 0, 0, 0],
            vec![1, 0, 0, 1, 1, 0],
            vec![0, 0, 1, 0, 0, 0],
            vec![0, 0, 1, 0, 0, 1],
            vec![0, 0, 0, 0, 1, 0],
        ],
        vec![
            vec![0, 1, 0, 0, 1, 0],
            vec![1, 0, 1, 0, 0, 0],
            vec![0, 1, 0, 1, 0, 0],
            vec![0, 0, 1, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0],
        ],
    ];

    for matrix in graphs.iter().cycle() {
        clear_screen();
        draw_ascii_graph(matrix);
        thread::sleep(Duration::from_secs(60)); // use 5 for testing
    }
}
