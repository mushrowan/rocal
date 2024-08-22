use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use chrono_tz::UTC;
use icalendar::{Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, Event};
use std::fs::{read_to_string, File};
use std::io::{Error, Read};
use std::path::{Path, PathBuf};

// Roadmap:
// make a bunch of timeblocks for each half hour segment in the cal.
// remove the timeblocks which intersect with events.
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
            println!("Event found: {:?}", &event.get_description().unwrap());
            if let (
                Some(DatePerhapsTime::DateTime(start_date_)),
                Some(DatePerhapsTime::DateTime(end_date_)),
            ) = (event.get_start(), event.get_end())
            {
                let start_datetime: NaiveDateTime =
                    start_date_.try_into_utc().unwrap().naive_local();
                let end_datetime: NaiveDateTime = end_date_.try_into_utc().unwrap().naive_local();
                if day == start_datetime.date() || day == end_datetime.date() {
                    events_on_target_date.push(event);
                }
            }
        }
    }
    events_on_target_date
}

fn main() -> Result<(), std::io::Error> {
    let today = Local::now().date_naive();
    let now = Local::now();
    let now_naive = Local::now().naive_local();
    dbg!(&today);
    dbg!(&now);
    dbg!(&now_naive);
    let st: NaiveTime = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let et: NaiveTime = NaiveTime::from_hms_opt(19, 0, 0).unwrap();
    let start_datetime = NaiveDateTime::new(today, st);
    let end_datetime = NaiveDateTime::new(today, et);
    let chunk_duration: TimeDelta = TimeDelta::minutes(30);
    let mut timeblocks = get_timeblocks(start_datetime, end_datetime, chunk_duration);
    let cal_dir = Path::new("/home/rowan/.calendar/ro");
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
        .map(|c| read_calendar_from_file(c))
        .collect::<Vec<_>>();

    let all_events_on_day = all_calendars
        .into_iter()
        .map(|c| get_events_on_day(today, c))
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

    Ok(())
}
