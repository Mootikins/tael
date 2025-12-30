//! Minimal raw mode test - press q to quit
use std::io::{self, Write};
// Use crossterm directly with use-dev-tty feature
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled},
};

fn main() -> io::Result<()> {
    println!("Enabling raw mode...");
    enable_raw_mode()?;

    let enabled = is_raw_mode_enabled()?;
    print!(
        "\rRaw mode enabled: {} - press 'q' to quit, any key to echo\r\n",
        enabled
    );
    io::stdout().flush()?;

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                print!("\rKey: {:?}                    \r\n", key);
                io::stdout().flush()?;
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    println!("\rRaw mode disabled, exiting.");
    Ok(())
}
