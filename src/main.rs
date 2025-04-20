use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

use console::style;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    event: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive (Serialize, Deserialize, Debug)]
struct Events {
    #[serde(flatten)]
    events: HashMap<String, DateTime<Utc>>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Add,
    List,
}

fn get_data_file() -> PathBuf {
    let config_dir = dirs::config_dir().expect("Could not find config dir");
    let path = config_dir.join("timesince").join("data.json");
    path
}

fn save_events(events: &HashMap<String, DateTime<Utc>>) {
    let path = get_data_file();
    let events_struct = Events {
        events: events.clone(),
    };
    let serialized = serde_json::to_string_pretty(&events_struct).expect("Failed to serialize data");

    fs::create_dir_all(path.parent().unwrap()).expect("Failed to create directories");
    fs::write(path, serialized).expect("Failed to write data to file");
}

fn load_events() -> HashMap<String, DateTime<Utc>> {
    let path = get_data_file();

    if !path.exists() {
        return HashMap::new();
    }

    let data = fs::read_to_string(path).expect("Failed to read your events");
    let events: Events = serde_json::from_str(&data).expect("Failed to parse your events file");
    events.events
}


fn human_readable(duration: Duration) -> String {
    let seconds = duration.num_seconds();

    if seconds < 60 {
        return format!("{} seconds ago", seconds);
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{} minutes ago", minutes);
    }

    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;
    if hours < 24 {
        if remaining_minutes > 0 {
            return format!("{} hours and {} minutes ago", hours, remaining_minutes);
        } else {
            return format!("{} hours ago", hours);
        }
    }

    let days = hours / 24;
    let remaining_hours = hours % 24;
    if remaining_hours > 0 {
        return format!("{} days and {} hours ago", days, remaining_hours);
    } else {
        return format!("{} days ago", days);
    }
}

fn print_duration(event_name: &String, timestamp: &DateTime<Utc>, pretty: bool) {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);
    if pretty {
        println!(
            "{} {} {}",
            style("You last did").bold(),
            style(event_name).green(),
            style(human_readable(duration)).bold()
        );
    } else {
        println!("{}: {}", style(event_name).bold().yellow(), human_readable(duration));
    }
}

fn set_event(event_name: &String, timestamp: &DateTime<Utc>) {
    let mut events = load_events();
    events.insert(event_name.clone(), *timestamp);
    save_events(&events);
}

fn main() {
    let args = Args::parse();

    match args.command {
        None => {
            let events = load_events();
            let event_name = args.event.expect("No event name provided");

            match events.get(&event_name) {
                Some(&timestamp) => {
                    print_duration(&event_name, &timestamp, true);
                }
                None => {
                    println!("Event '{}' not found. You can add it using the 'add' command", event_name)
                }
            }
        }
        Some(Command::List) => {
            let events = load_events();
            if events.is_empty() {
                println!("No events found.");
            } else {
                println!("Events:");
                for (event_name, timestamp) in events.iter() {
                    print_duration(event_name, timestamp, false);
                }
            }
        }
        Some(Command::Add) => {
            let event_name = args.event.expect("No event name provided");
            set_event(&event_name, &Utc::now());
            println!("Added {}", event_name);
        }
    }
}