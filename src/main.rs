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
    Mark {
        #[arg(help = "The name of the existing event to mark as done")]
        event: String,
    },

    #[command(about = "Remove an event")]
    Remove {
        #[arg(help = "The name of the event to remove")]
        event: String,
    }
}

struct DataStore {
    path: PathBuf,
}

impl DataStore {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn default() -> Self {
        let config_dir = dirs::config_dir().expect("Could not find config dir");
        let path = config_dir.join("timesince").join("data.json");
        Self::new(path)
    }

    fn load(&self) -> HashMap<String, DateTime<Utc>> {
        if !self.path.exists() {
            return HashMap::new();
        }

        let data = fs::read_to_string(&self.path).expect("Failed to read your events");
        let events: Events = serde_json::from_str(&data).expect("Failed to parse your events file");
        events.events
    }

    fn save(&self, events: &HashMap<String, DateTime<Utc>>) {
        let events_struct = Events {
            events: events.clone(),
        };
        let serialized =
            serde_json::to_string_pretty(&events_struct).expect("Failed to serialize data");

        fs::create_dir_all(self.path.parent().unwrap()).expect("Failed to create directories");
        fs::write(&self.path, serialized).expect("Failed to write data to file");
    }
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

fn mark_event(datastore: &DataStore, event_name: &String) {
    let mut events = datastore.load();
    if !events.contains_key(event_name) {
        println!("Event '{}' not found. You can add it using the 'add' command", event_name);
        return;
    }
    events.insert(event_name.clone(), Utc::now());
    datastore.save(&events);
    println!("{} '{}', updated!", "✅", style(event_name).underlined());
}

fn add_event(datastore: &DataStore, event_name: &String, timestamp: DateTime<Utc>) {
    let mut events = datastore.load();
    if events.contains_key(event_name) {
        println!("Event '{}' already exists. Use 'mark' to update it.", event_name);
        return;
    }

    events.insert(event_name.clone(), timestamp);
    datastore.save(&events);
    println!("{} '{}' added!", style("➕").green(), style(event_name).underlined());
}

fn remove_event(datastore: &DataStore, event_name: &String) {
    let mut events = datastore.load();
    if events.remove(event_name).is_some() {
        datastore.save(&events);
        println!("{} '{}' removed!", style("🗑").bold().red(), style(event_name).underlined());
    } else {
        println!(
            "'{}' {}",
            style(event_name).italic().yellow(),
            style("not found.").red()
        );
    }
}

fn show_time_since(datastore: &DataStore, event_name: String) {
    let events = datastore.load();
    match events.get(&event_name) {
        Some(&timestamp) => {
            print_duration(&event_name, &timestamp, true);
        }
        None => {
            println!("Event '{}' not found. You can add it using the 'add' command", event_name);
        }
    }
}

fn show_all_events(datastore: &DataStore) {
    let events = datastore.load();
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
    let datastore = DataStore::default();

    match args.command {
        None => {
            let event_name = args.event.expect("Need an event name");
            show_time_since(&datastore, event_name);
        }
        Some(Command::List) => {
            show_all_events(&datastore);
        }
        Some(Command::Add { event: name }) => {
            add_event(&datastore, &name, Utc::now());
        }
        Some(Command::Mark { event: name }) => {
            mark_event(&datastore, &name);
        }
        Some(Command::Remove { event: name }) => {
            remove_event(&datastore, &name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_datastore(file_name: &str) -> DataStore {
        let temp_dir = tempdir().expect("Could not create temp dir");
        let path = temp_dir.path().join(file_name);
        DataStore::new(path)
    }

    #[test]
    fn test_human_readable() {
        let duration = Duration::seconds(3600);
        assert_eq!(human_readable(duration), "1 hour");
    }

    #[test]
    fn test_add_event() {
        let datastore = test_datastore("add.json");

        let event_name = "test_event".to_string();
        let timestamp = Utc::now();

        add_event(&datastore, &event_name, timestamp);

        let events = datastore.load();

        assert_eq!(events.get(&event_name), Some(&timestamp));
    }

    #[test]
    fn test_remove_event() {
        let datastore = test_datastore("remove.json");

        let event_name_a = "event_a".to_string();
        let event_name_b = "event_b".to_string();

        let timestamp = Utc::now();

        add_event(&datastore, &event_name_a, timestamp);
        add_event(&datastore, &event_name_b, timestamp);

        remove_event(&datastore, &event_name_b);

        let events = datastore.load();

        assert_eq!(events.get(&event_name_a), Some(&timestamp));
        assert_eq!(events.get(&event_name_b), None);
    }

    #[test]
    fn test_mark_event() {
        let datastore = test_datastore("mark.json");

        let event_name = "mark_test".to_string();

        add_event(&datastore, &event_name, Utc::now() - Duration::days(10));

        mark_event(&datastore, &event_name);

        let events = datastore.load();
        let now = Utc::now();
        let updated = events.get(&event_name).unwrap();

        assert!((now.signed_duration_since(*updated)).num_seconds() < 5);
    }
}
