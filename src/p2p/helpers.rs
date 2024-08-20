use std::io;

use chrono::Utc;
use colored::Colorize;

pub fn show_p2p_options() {
    println!();
    println!("{}", "1.Push Ixs".cyan().bold());
    println!("{}", "2.Announce Ixs len".yellow().bold());
    println!();
}

pub fn read_p2p_address() -> Result<String, io::Error> {
    let mut buf = String::new();
    println!("{}", "Enter the address of the peer:".green().bold());
    read(&mut buf)?;
    Ok(buf.trim().to_string())
}

pub fn read(buf: &mut String) -> Result<(), io::Error> {
    std::io::stdin().read_line(buf)?;
    Ok(())
}

pub fn get_unix_timestamp_secs() -> u64 {
    let now = Utc::now();
    (now.timestamp_millis() / 1000i64) as u64
}
