use chrono::{DateTime, Duration, Utc};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use console::style;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use strsim::jaro_winkler;

#[derive(Parser, Debug)]
#[command(
    version,
    about="A CLI tool to track how long it's been since you last did something",
    long_about = "Timesince helps you record events (like 'workout', 'meditate') and then check how long it's been since you did them.",
    override_usage = "\ntimesince <EVENT>\ntimesince <COMMAND> <EVENT>"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(help = "The event name to query (e.g., 'reading')")]
    event: Option<String>,
}

#[derive (Serialize, Deserialize, Debug)]
struct Events {
    #[serde(flatten)]
    events: HashMap<String, DateTime<Utc>>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "List all events", long_about = "Displays all tracked events with time since they were last updated")]
    List,

    #[command(about = "Record that you did something", long_about = "Records an event as done now, creating it if it doesn't exist")]
    Did {
        #[arg(help = "The name of the event you did")]
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

fn find_closest_event<'a>(name: &str, events: &'a HashMap<String, DateTime<Utc>>) -> Option<&'a String> {
    let threshold = 0.8;
    events
        .keys()
        .map(|key| (key, jaro_winkler(&name.to_lowercase(), &key.to_lowercase())))
        .filter(|(_, score)| *score >= threshold)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(key, _)| key)
}

fn staleness_style(duration: &Duration) -> console::Style {
    let days = duration.num_days();
    if days < 1 {
        console::Style::new().green()
    } else if days <= 7 {
        console::Style::new().yellow()
    } else {
        console::Style::new().red()
    }
}

fn print_duration(event_name: &String, timestamp: &DateTime<Utc>, pretty: bool) {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);
    let duration_text = human_readable(duration);
    if pretty {
        println!(
            "{} {} {}",
            style("Time since last").bold(),
            style(event_name).green(),
            staleness_style(&duration).bold().apply_to(&duration_text)
        );
    } else {
        println!("{}: {}", style(event_name).bold().yellow(), staleness_style(&duration).apply_to(&duration_text));
    }
}

fn did_event(datastore: &DataStore, event_name: &String) {
    let mut events = datastore.load();
    let is_new = !events.contains_key(event_name);
    events.insert(event_name.clone(), Utc::now());
    datastore.save(&events);
    if is_new {
        println!("{} '{}', done!", "✅", style(event_name).underlined());
    } else {
        println!("{} '{}', updated!", "✅", style(event_name).underlined());
    }
}

fn remove_event(datastore: &DataStore, event_name: &String) {
    let mut events = datastore.load();
    if events.remove(event_name).is_some() {
        datastore.save(&events);
        println!("{} '{}' removed!", style("🗑").bold().red(), style(event_name).underlined());
    } else {
        let events = datastore.load();
        if let Some(suggestion) = find_closest_event(event_name, &events) {
            println!(
                "'{}' {} Did you mean '{}'?",
                style(event_name).italic().yellow(),
                style("not found.").red(),
                style(suggestion).green()
            );
        } else {
            println!(
                "'{}' {}",
                style(event_name).italic().yellow(),
                style("not found.").red()
            );
        }
    }
}

fn show_time_since(datastore: &DataStore, event_name: String) {
    let events = datastore.load();
    match events.get(&event_name) {
        Some(&timestamp) => {
            print_duration(&event_name, &timestamp, true);
        }
        None => {
            if let Some(suggestion) = find_closest_event(&event_name, &events) {
                println!("Event '{}' not found. Did you mean '{}'?", event_name, style(suggestion).green());
            } else {
                println!("Event '{}' not found. You can track it using 'timesince did {}'", event_name, event_name);
            }
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

        let mut sorted_events: Vec<_> = events.iter().collect();
        sorted_events.sort_by(|a, b| b.1.cmp(a.1));

        let now = Utc::now();
        for (event_name, timestamp) in sorted_events {
            let duration = now.signed_duration_since(timestamp);
            let color = staleness_style(&duration);
            table.add_row(vec![
                Cell::new(event_name),
                Cell::new(color.apply_to(human_readable(duration))),
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
        Some(Command::Did { event: name }) => {
            did_event(&datastore, &name);
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
    fn test_remove_event() {
        let datastore = test_datastore("remove.json");

        let event_name_a = "event_a".to_string();
        let event_name_b = "event_b".to_string();

        did_event(&datastore, &event_name_a);
        did_event(&datastore, &event_name_b);

        remove_event(&datastore, &event_name_b);

        let events = datastore.load();

        assert!(events.contains_key(&event_name_a));
        assert_eq!(events.get(&event_name_b), None);
    }

    #[test]
    fn test_did_event() {
        let datastore = test_datastore("did.json");

        let event_name = "did_test".to_string();

        did_event(&datastore, &event_name);

        let events = datastore.load();
        let now = Utc::now();
        let updated = events.get(&event_name).unwrap();

        assert!((now.signed_duration_since(*updated)).num_seconds() < 5);
    }

    #[test]
    fn test_find_closest_event_match() {
        let mut events = HashMap::new();
        events.insert("workout".to_string(), Utc::now());
        events.insert("meditation".to_string(), Utc::now());

        let result = find_closest_event("workut", &events);
        assert_eq!(result, Some(&"workout".to_string()));
    }

    #[test]
    fn test_find_closest_event_no_match() {
        let mut events = HashMap::new();
        events.insert("workout".to_string(), Utc::now());

        let result = find_closest_event("zzzzzzz", &events);
        assert_eq!(result, None);
    }

    #[test]
    fn test_staleness_style() {
        // Green for < 1 day
        let recent = Duration::hours(12);
        let s = staleness_style(&recent);
        assert_eq!(format!("{}", s.apply_to("x")), format!("{}", console::Style::new().green().apply_to("x")));

        // Yellow for 1-7 days
        let mid = Duration::days(3);
        let s = staleness_style(&mid);
        assert_eq!(format!("{}", s.apply_to("x")), format!("{}", console::Style::new().yellow().apply_to("x")));

        // Red for > 7 days
        let old = Duration::days(14);
        let s = staleness_style(&old);
        assert_eq!(format!("{}", s.apply_to("x")), format!("{}", console::Style::new().red().apply_to("x")));
    }

    #[test]
    fn test_did_event_auto_creates() {
        let datastore = test_datastore("did_auto.json");

        let event_name = "new_event".to_string();
        did_event(&datastore, &event_name);

        let events = datastore.load();
        assert!(events.contains_key(&event_name));

        let now = Utc::now();
        let timestamp = events.get(&event_name).unwrap();
        assert!((now.signed_duration_since(*timestamp)).num_seconds() < 5);
    }
}
