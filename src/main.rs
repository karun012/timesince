use chrono::{DateTime, Duration, Utc};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use console::style;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
    version,
    about="A CLI tool to track how long it's been since you last did something",
    long_about = "Timesince helps you record events (like 'workout', 'meditate') and then check how long it's been since you did them."
)]
struct Args {
    #[arg(help = "The event name to query (e.g., 'reading')")]
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
    #[command(about = "Add a new event", long_about = "Adds a new event and sets its timestamp to now")]
    Add {
        #[arg(help = "The name of the event to add")]
        event: String
    },

    #[command(about = "List all events", long_about = "Displays all tracked events with time since they were last updated")]
    List,

    #[command(about = "Mark an existing event as done now", long_about = "Updates the timestamp for an existing event to now")]
    Did {
        #[arg(help = "The name of the existing event to mark as done")]
        event: String,
    },

    #[command(about = "Remove an event")]
    Remove {
        #[arg(help = "The name of the event to remove")]
        event: String,
    }
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
    let rounded = Duration::seconds(seconds);
    HumanTime::from(rounded).to_text_en(chrono_humanize::Accuracy::Precise, chrono_humanize::Tense::Present)
}

fn print_duration(event_name: &String, timestamp: &DateTime<Utc>, pretty: bool) {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);
    if pretty {
        println!(
            "{} {} {}",
            style("Time since last").bold(),
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

fn add_event(event_name: &String, timestamp: DateTime<Utc>) {
    let mut events = load_events();
    match events.get(event_name) {
        Some(_) => {
            println!("Event '{}' already exists. Use 'did' to update it.", event_name);
            return;
        }
        None => {
            events.insert(event_name.clone(), timestamp);
            save_events(&events);
        }
    }
    
    events.insert(event_name.clone(), timestamp);
    save_events(&events);
}

fn remove_event(event_name: &String) {
    let mut events = load_events();
    if events.remove(event_name).is_some() {
        save_events(&events);
        println!("{} '{}' removed!", style("🗑").bold().red(), style(event_name).underlined());
    } else {
        println!(
            "'{}' {}",
            style(event_name).italic().yellow(),
            style("not found.").red()
        );
    }
}

fn show_time_since(event_name: String) {
    let events = load_events();
    match events.get(&event_name) {
        Some(&timestamp) => {
            print_duration(&event_name, &timestamp, true);
        }
        None => {
            println!("Event '{}' not found. You can add it using the 'add' command", event_name)
        }
    }
}

fn show_all_events() {
    let events = load_events();
    if events.is_empty() {
        println!("No events found.");
    } else {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Event").add_attribute(Attribute::Bold),
                Cell::new("Last Done").add_attribute(Attribute::Bold),
            ]);

        for (event_name, timestamp) in events.iter() {
            let now = Utc::now();
            let duration = now.signed_duration_since(timestamp);
            table.add_row(vec![
                Cell::new(event_name),
                Cell::new(human_readable(duration)),
            ]);
        }

        println!("{table}");
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        None => {
            let event_name = args.event.expect("Need an event name");
            show_time_since(event_name);
        }
        Some(Command::List) => {
            show_all_events();
        }
        Some(Command::Add { event: name }) => {
            add_event(&name, Utc::now());
            println!("{} '{}' added!", style("➕").bold().green(), style(name).underlined());
        }

        Some(Command::Did { event: name, .. }) => {
            set_event(&name, &Utc::now());
            println!("{} '{}', updated!", style("✅").bold().blue(), style(name).underlined());
        }
        Some(Command::Remove { event: name}) => {
            remove_event(&name);
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_human_readable() {
        let duration = Duration::seconds(3600);
        assert_eq!(human_readable(duration), "1 hour");
    }

    #[test]
    fn test_add_event() {
        let _ = fs::remove_file(get_data_file());
        
        let event_name = "test_event".to_string();
        let timestamp = Utc::now();

        add_event(&event_name, timestamp);
        
        let events = load_events();

        assert_eq!(events.get(&event_name), Some(&timestamp));
    }
    
    #[test]
    fn test_remove_event() {
        let _ = fs::remove_file(get_data_file());
        
        let event_name_a = "event_a".to_string();
        let event_name_b = "event_b".to_string();
        
        let timestamp = Utc::now();

        add_event(&event_name_a, timestamp);
        add_event(&event_name_b, timestamp);
        
        remove_event(&event_name_b);

        let events = load_events();

        assert_eq!(events.get(&event_name_a), Some(&timestamp));
        assert_eq!(events.get(&event_name_b), None);
    }
}