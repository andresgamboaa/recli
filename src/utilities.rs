use std::fs;

use chrono::{Utc, DateTime};
use serde_derive::Deserialize;

use crate::Command;

pub fn get_current_date() -> DateTime<Utc> {
    let current_utc_time: DateTime<Utc> = Utc::now();
    let current_date = current_utc_time.date_naive();
    let date_with_zero_time = current_date.and_hms_opt(0, 0, 0);

    DateTime::from_utc(date_with_zero_time.unwrap(), Utc)
}

#[derive(Deserialize)]
struct CommandsVec {
    commands: Vec<Command>,
}

pub fn get_commands_from_toml(filename: &str) -> Vec<Command> {

    let contents = fs::read_to_string(filename).unwrap_or_else(|error| {
        panic!("Cound not read commands file: {error}");
    });

    let commands_vec : CommandsVec = toml::from_str(&contents).unwrap_or_else(|error: toml::de::Error| {
        panic!("Invalid toml file: {}", error);
    });
    
    commands_vec.commands
}