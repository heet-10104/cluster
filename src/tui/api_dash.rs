use crossterm::{
    cursor, execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::{
    io::{stdout, Write},
    thread::sleep,
    time::Duration,
};

pub fn render_api_dash() -> std::io::Result<()> {
    let mut stdout = stdout();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let mut tick = 0;

    loop {
        let fake_data = get_fake_status_data(tick);

        // Clear screen and draw
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
        println!("📊 API Status Dashboard\n");

        for (url, status) in fake_data {
            let icon = match status.as_str() {
                "Live" => "✅",
                "Error" => "❌",
                _ => "⚠️",
            };
            println!("{icon}  {url}");
        }

        tick += 1;
        sleep(Duration::from_secs(2));
    }

    // Cleanup on exit (never reached in this example)
    // disable_raw_mode()?;
    // execute!(stdout, LeaveAlternateScreen)?;
    // Ok(())
}

fn get_fake_status_data(tick: usize) -> Vec<(String, String)> {
    vec![
        (
            "https://api.github.com".to_string(),
            if tick % 2 == 0 { "Live" } else { "Error" }.into(),
        ),
        ("https://example.com".to_string(), "Live".into()),
        ("https://httpstat.us/503".to_string(), "Error".into()),
    ]
}
