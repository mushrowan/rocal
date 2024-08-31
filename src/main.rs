use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use config::{Config, ConfigError};
use dirs::home_dir;
use futures::executor::block_on;
use http::Uri;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::connect::HttpConnector;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, Event, EventLike};
use inquire::{DateSelect, Select, Text};
use libdav::{
    auth::{Auth, Password},
    dav::{WebDavClient, WebDavError},
    sd::{find_context_url, BootstrapError},
    CalDavClient,
};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

// Roadmap:
// for all the remaining timeblocks, prompt for a thing to do - suggest tasks.
// once done, add all to the calendar.
fn get_timeblocks(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    chunk_duration: TimeDelta,
) -> Vec<TimeBlock> {
    let mut current_chunk = TimeBlock::new(start_time, chunk_duration);
    let mut timeblocks = vec![];
    while current_chunk.end_time <= end_time {
        timeblocks.push(current_chunk);
        current_chunk = TimeBlock::new(current_chunk.start_time + chunk_duration, chunk_duration);
    }
    timeblocks
}

#[derive(Copy, Clone, Debug)]
struct TimeBlock {
    start_time: NaiveDateTime,
    block_duration: TimeDelta,
    end_time: NaiveDateTime,
}
impl TimeBlock {
    fn new(start: NaiveDateTime, duration: TimeDelta) -> Self {
        Self {
            start_time: start,
            block_duration: duration,
            end_time: start + duration,
        }
    }
}

fn event_intersects_with_timeblock(timeblock: TimeBlock, event: &Event) -> bool {
    match (event.get_start(), event.get_end()) {
        (Some(DatePerhapsTime::DateTime(es_)), Some(DatePerhapsTime::DateTime(ee_))) => {
            // I don't know if there's an easier way to do this, but try_into_utc seems to be funky
            // here. but maybe i'm messing stuff up a lot.
            // Basically we're assuming that the calendar events are in local time (probably not
            // the best assumption, but it's true for all my calendar events.)
            let es_utc: DateTime<Utc> = es_.try_into_utc().unwrap();
            let ee_utc: DateTime<Utc> = ee_.try_into_utc().unwrap();
            let es_local: DateTime<Local> = DateTime::from(es_utc);
            let ee_local: DateTime<Local> = DateTime::from(ee_utc);
            let es: NaiveDateTime = es_local.naive_local();
            let ee: NaiveDateTime = ee_local.naive_local();

            (timeblock.start_time <= es && es < timeblock.end_time)
                || (timeblock.start_time < ee && ee <= timeblock.end_time)
                || (es <= timeblock.start_time && timeblock.end_time <= ee)
        }
        _ => false,
    }
}

fn remove_intersecting_segments(event: &Event, mut timeblocks: Vec<TimeBlock>) -> Vec<TimeBlock> {
    timeblocks.retain(|&block| !event_intersects_with_timeblock(block, event));
    timeblocks
}

fn read_calendar_from_file(cf: PathBuf) -> Calendar {
    let cal_contents: String = read_to_string(cf).unwrap();
    let cal: Calendar = cal_contents.parse::<Calendar>().unwrap();
    cal
}

async fn create_caldav_client(
    uri: String,
    user: String,
    pass: String,
) -> Result<CalDavClient<HttpsConnector<HttpConnector>>, BootstrapError> {
    let uri = uri.parse::<Uri>().unwrap();
    let auth = Auth::Basic {
        username: user,
        password: Some(Password::from(pass)),
    };

    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .expect("no native root CA certificates found")
        .https_or_http()
        .enable_http1()
        .build();
    let webdav = WebDavClient::new(uri, auth, https);
    // Optionally, perform bootstrap sequence here.
    // CalDavClient::new(webdav).unwrap()
    CalDavClient::new_via_bootstrap(webdav).await
}

fn menu() -> Result<(), Box<dyn Error>> {
    let main_options = vec!["plan", "sync", "config", "quit"];

    let options = Select::new("Select a menu option: ", main_options).prompt()?;
    Ok(())
}

fn get_events_on_day(day: NaiveDate, cal: Calendar) -> Vec<Event> {
    let mut events_on_target_date: Vec<Event> = Vec::new();
    for component in cal.components {
        if let CalendarComponent::Event(event) = component {
            if let (Some(DatePerhapsTime::DateTime(sd_)), Some(DatePerhapsTime::DateTime(ed_))) =
                (event.get_start(), event.get_end())
            {
                let sd_utc: DateTime<Utc> = sd_
                    .try_into_utc()
                    .expect("Couldnt convert event time into utc.");
                let ed_utc: DateTime<Utc> = ed_
                    .try_into_utc()
                    .expect("Couldnt convert event time into utc.");
                let sd_local: DateTime<Local> = DateTime::from(sd_utc);
                let ed_local: DateTime<Local> = DateTime::from(ed_utc);
                let sd: NaiveDateTime = sd_local.naive_local();
                let ed: NaiveDateTime = ed_local.naive_local();
                if sd.date() == day || ed.date() == day {
                    events_on_target_date.push(event);
                }
            }
        }
    }
    events_on_target_date
}

fn try_build_settings() {}

fn try_deserialize_settings(path: &str) -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(config::File::with_name(path))
        .build()
        .unwrap()
        .try_deserialize::<HashMap<String, String>>()
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. check if there is a config. if no config, prompt for requireds.
    // 2. if any requireds missing from existing config, prompt for those.
    // 3. if there is a config for remote cals, attempt to fetch the given
    // remote cals from given hrefs.
    // 4. if succeeded, add remote cals to a vec of remote cals successfully
    // accessed. if not, warn.
    let settings_result = try_deserialize_settings(&"src/config.toml");
    if let Ok(settings) = settings_result {
        println!("Settings successfully deserialized");
    }
    match settings_result {
        Ok(settings) => {}
        Err(_) => {
            panic!("shit is fuckeddddd");
        }
    }

    // probably a better way to do this using borrowing, but works for now.
    let uri = settings["calendar_url"].clone();
    let username = settings["calendar_username"].clone();
    let password = settings["calendar_password"].clone();

    println!("welcome to rocal!");
    menu();

    let client = create_caldav_client(uri, username, password).await?;

    // Finding the user principal, e.g. /ro/
    let user_principal = client
        .find_current_user_principal()
        .await
        .expect("No current user principal found, or 404 returned.")
        .expect("unable to find current user principal");
    let calendar_home_set = client.find_calendar_home_set(&user_principal).await?;
    // Assume that the first home set is the right one.
    let first_calendar_home_set = calendar_home_set.first().unwrap();
    let existing_calendars = client
        .find_calendars(&first_calendar_home_set)
        .await
        .unwrap();
    // .map(|foundres| foundres.href);
    let mut calendar_hrefs_in_home_set = vec![];
    for cal in existing_calendars {
        existing_calendars.push(cal.href);
    }
    println!("{:?}", existing_calendars);

    // Testing function which creates a calendar. Breaks if the calendar
    // already exists.
    // client
    //     .create_calendar(format!("{}{}/", first_calendar_home_set.path(), "testing"))
    //     .await?;

    let day = DateSelect::new("When do you want to plan for?")
        .with_starting_date(Local::today().naive_local())
        .with_week_start(chrono::Weekday::Sun)
        // .with_help_message("Possible flights will be displayed according to the selected date")
        .prompt()
        .expect("prompting for date failed. somehow.");
    let st: NaiveTime = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let et: NaiveTime = NaiveTime::from_hms_opt(19, 0, 0).unwrap();
    let start_datetime = NaiveDateTime::new(day, st);
    let end_datetime = NaiveDateTime::new(day, et);
    let chunk_duration: TimeDelta = TimeDelta::minutes(30);
    let mut timeblocks = get_timeblocks(start_datetime, end_datetime, chunk_duration);
    // let cal_dir = Path::new("/home/rain/.calendar/ro");
    let mut cal_dir = home_dir().expect("unable to get home directory.");
    cal_dir.push(".calendar");
    cal_dir.push("ro");

    let cal_dir_contents = cal_dir
        .read_dir()
        .expect("read_dir on calendar directory path failed")
        .map(|p| p.expect("failed to get Direntry").path())
        .collect::<Vec<_>>();

    let all_calendars = cal_dir_contents
        .into_iter()
        .map(read_calendar_from_file)
        .collect::<Vec<_>>();

    let all_events_on_day = all_calendars
        .into_iter()
        .map(|c| get_events_on_day(day, c))
        .collect::<Vec<_>>()
        .concat();

    for event in &all_events_on_day {
        timeblocks = remove_intersecting_segments(event, timeblocks);
    }

    // Day planning bit
    let mut plan_chunks: Vec<Event> = vec![];
    let mut blocks_without_break: u8 = 0;
    for block in &timeblocks {
        if blocks_without_break >= 4 {
            println!(
                "You have gone {:?} blocks without a break.",
                blocks_without_break
            );
            println!("Take a break soon! Name a block \"break\" to reset the break count.");
        }
        let prompt = format!(
            "Enter a task for {:?}-{:?}: ",
            block.start_time.time(),
            block.end_time.time()
        );
        let block_name = Text::new(&prompt).prompt().unwrap();
        if block_name == "break" {
            blocks_without_break = 0;
        } else {
            blocks_without_break += 1;
        }
        let mut chunk = Event::new();
        chunk.summary(&block_name);
        chunk.starts(block.start_time);
        chunk.ends(block.end_time);
        plan_chunks.push(chunk);
    }

    let day_plan: Calendar = plan_chunks.into_iter().collect::<Calendar>();
    // let mut debug_output_cal = cal_dir.clone();
    // debug_output_cal.push("test_output_cal.ics");
    let debug_output_cal = "./output_cal.ics";
    let ics = write(debug_output_cal, format!("{}", day_plan));

    Ok(())
}
