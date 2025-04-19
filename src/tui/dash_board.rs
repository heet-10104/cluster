use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use rand::Rng;
use std::io::stdout;
use std::{thread, time::Duration};

pub struct Metrics {
    pub cpu: f64,
    pub ram: f64,
    pub netspeed: (f64, f64),
    pub net_history: (Vec<f64>, Vec<f64>), // (upload history, download history)
}

fn clear_screen() {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
}

fn draw_bar_chart(values: &[f64], max_val: f64) -> String {
    let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    values
        .iter()
        .map(|&v| {
            let scaled = (v / max_val * (bars.len() as f64 - 1.0)).round();
            let idx = scaled.clamp(0.0, (bars.len() - 1) as f64) as usize;
            bars[idx]
        })
        .collect()
}

fn format_metrics(metrics: &Metrics, title: &str) -> Vec<String> {
    let (upload_hist, download_hist) = &metrics.net_history;
    let max_up = upload_hist.iter().cloned().fold(0.0, f64::max).max(1.0);
    let max_down = download_hist.iter().cloned().fold(0.0, f64::max).max(1.0);

    let lines = vec![
        format!("{:^30}", title),
        format!("{:-<30}", ""),
        format!("CPU:     {:>6.1} %", metrics.cpu),
        format!("RAM:     {:>6.2} GB", metrics.ram),
        format!("NetSpeed:"),
        format!(
            "  Upload:   {:>6.2} Mbps   {}",
            metrics.netspeed.0,
            draw_bar_chart(upload_hist, max_up)
        ),
        format!(
            "  Download: {:>6.2} Mbps   {}",
            metrics.netspeed.1,
            draw_bar_chart(download_hist, max_down)
        ),
    ];

    // Padding to ensure all columns are the same height
    let target_height = 8;
    let mut padded = lines;
    while padded.len() < target_height {
        padded.push("".to_string());
    }

    padded
}

fn merge_columns(columns: Vec<Vec<String>>) -> Vec<String> {
    let height = columns.iter().map(|col| col.len()).max().unwrap_or(0);
    (0..height)
        .map(|i| {
            columns
                .iter()
                .map(|col| col.get(i).cloned().unwrap_or_else(|| " ".repeat(30)))
                .collect::<Vec<_>>()
                .join("    ") // spacing between systems
        })
        .collect()
}

pub fn render_dash() {
    let mut upload_histories = vec![vec![0.0; 10]; 3];
    let mut download_histories = vec![vec![0.0; 10]; 3];

    loop {
        let mut columns = Vec::new();

        for i in 0..3 {
            let cpu = rand_float(10.0, 90.0);
            let ram = rand_float(4.0, 16.0);
            let upload = rand_float(0.0, 10.0);
            let download = rand_float(0.0, 50.0);

            let upload_hist = &mut upload_histories[i];
            let download_hist = &mut download_histories[i];

            upload_hist.push(upload);
            if upload_hist.len() > 10 {
                upload_hist.remove(0);
            }

            download_hist.push(download);
            if download_hist.len() > 10 {
                download_hist.remove(0);
            }

            let metrics = Metrics {
                cpu,
                ram,
                netspeed: (upload, download),
                net_history: (upload_hist.clone(), download_hist.clone()),
            };

            columns.push(format_metrics(&metrics, &format!("System {}", i + 1)));
        }

        clear_screen();
        for line in merge_columns(columns) {
            println!("{}", line);
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn rand_float(min: f64, max: f64) -> f64 {
    let mut rng = rand::rng();
    rng.random_range(min..max)
}
