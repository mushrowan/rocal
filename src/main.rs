use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use dirs::home_dir;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, Event, EventLike};
use inquire::{DateSelect, Text};
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};

// Roadmap:
// for all the remaining timeblocks, prompt for a thing to do - suggest tasks.
// once done, add all to the calendar.
fn get_timeblocks(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    chunk_duration: TimeDelta,
) -> Vec<[NaiveDateTime; 2]> {
    let mut current_chunk_endtime = start_time + chunk_duration;
    let mut timeblocks = vec![];
    while current_chunk_endtime <= end_time {
        timeblocks.push([
            current_chunk_endtime - chunk_duration,
            current_chunk_endtime,
        ]);
        current_chunk_endtime += chunk_duration;
    }
    timeblocks
}

fn event_intersects_with_timeblock(timeblock: [NaiveDateTime; 2], event: &Event) -> bool {
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

            (timeblock[0] <= es && es < timeblock[1])
                || (timeblock[0] < ee && ee <= timeblock[1])
                || (es <= timeblock[0] && timeblock[1] <= ee)
        }
        _ => false,
    }
}

fn remove_intersecting_segments(
    event: &Event,
    mut timeblocks: Vec<[NaiveDateTime; 2]>,
) -> Vec<[NaiveDateTime; 2]> {
    timeblocks.retain(|&block| !event_intersects_with_timeblock(block, event));
    timeblocks
}

fn read_calendar_from_file(cf: PathBuf) -> Calendar {
    let cal_contents: String = read_to_string(cf).unwrap();
    let cal: Calendar = cal_contents.parse::<Calendar>().unwrap();
    cal
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

fn main() -> Result<(), std::io::Error> {
    println!("welcome to rocal!");
    let tomorrow = Local::now().date_naive() + TimeDelta::days(1);
    let st: NaiveTime = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let et: NaiveTime = NaiveTime::from_hms_opt(19, 0, 0).unwrap();
    let start_datetime = NaiveDateTime::new(tomorrow, st);
    let end_datetime = NaiveDateTime::new(tomorrow, et);
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

    for cal_file in &cal_dir_contents {
        println!("cal file: {:?}", cal_file)
    }

    let all_calendars = cal_dir_contents
        .into_iter()
        //.filter(|p| p.ends_with(".ics"))
        //.collect::<Vec<_>>()
        //.into_iter()
        .map(read_calendar_from_file)
        .collect::<Vec<_>>();

    let all_events_on_day = all_calendars
        .into_iter()
        .map(|c| get_events_on_day(tomorrow, c))
        .collect::<Vec<_>>()
        .concat();
    // debug
    for event in &all_events_on_day {
        timeblocks = remove_intersecting_segments(event, timeblocks);
    }
    println!("Timeblocks after removing intersecting segments:");
    for line in &timeblocks {
        println!("{}, {}", line[0], line[1]);
    }
    let mut plan_chunks: Vec<Event> = vec![];
    let mut blocks_without_break: u8 = 0;
    let date_selector = DateSelect::new("When do you want to plan for?")
        .with_starting_date(Local::today().naive_local())
        .with_week_start(chrono::Weekday::Sun)
        // .with_help_message("Possible flights will be displayed according to the selected date")
        .prompt()
        .expect("prompting for date failed. somehow.");
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
            block[0].time(),
            block[1].time()
        );
        let block_name = Text::new(&prompt).prompt().unwrap();
        if block_name == "break" {
            blocks_without_break = 0;
        } else {
            blocks_without_break += 1;
        }
        let mut chunk = Event::new();
        chunk.summary(&block_name);
        chunk.starts(block[0]);
        chunk.ends(block[1]);
        plan_chunks.push(chunk);
    }
    let day_plan: Calendar = plan_chunks.into_iter().collect::<Calendar>();
    let mut debug_output_cal = cal_dir.clone();
    debug_output_cal.push("test_output_cal.ics");
    let _ = write(debug_output_cal, format!("{}", day_plan));

    Ok(())
}
