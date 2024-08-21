use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use icalendar::{Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, Event};
use std::fs::File;
use std::io::{Error, Read};
use std::path::Path;

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
            match (es_, ee_) {
                (CalendarDateTime::Floating(es), CalendarDateTime::Floating(ee)) => {
                    (timeblock[0] < ee && ee < timeblock[1])
                        || (timeblock[0] < es && es < timeblock[1])
                        || (es <= timeblock[0] && timeblock[1] <= ee)
                }
                _ => false,
            }
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

fn read_calendar_from_file(mut cf: File) -> Calendar {
    let mut cal_contents = String::new();
    cf.read_to_string(&mut cal_contents);
    let cal: Calendar = cal_contents.parse::<Calendar>().unwrap();
    cal
}

fn get_events_on_day(day: NaiveDate, cal: Calendar) -> Vec<Event> {
    let mut events_on_target_date: Vec<Event> = Vec::new();
    for component in cal.components {
        if let CalendarComponent::Event(event) = component {
            if let (
                Some(DatePerhapsTime::DateTime(start_date_)),
                Some(DatePerhapsTime::DateTime(end_date_)),
            ) = (event.get_start(), event.get_end())
            {
                if let (
                    CalendarDateTime::Floating(start_date),
                    CalendarDateTime::Floating(end_date),
                ) = (start_date_, end_date_)
                {
                    if day == start_date.date() || day == end_date.date() {
                        events_on_target_date.push(event);
                    }
                }
            }
        }
    }
    events_on_target_date
}

fn main() -> Result<(), std::io::Error> {
    let today = Local::now().date_naive();
    let st: NaiveTime = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let et: NaiveTime = NaiveTime::from_hms_opt(19, 0, 0).unwrap();
    let start_datetime = NaiveDateTime::new(today, st);
    let end_datetime = NaiveDateTime::new(today, et);
    let chunk_duration: TimeDelta = TimeDelta::minutes(30);
    let mut timeblocks = get_timeblocks(start_datetime, end_datetime, chunk_duration);
    for line in timeblocks {
        println!("{}, {}", line[0], line[1]);
    }
    let cal_path = Path::new("/home/rain/.calendar/rowan/YLWN7J10GUMCFWZOF4LXD82YEVP7F18W0JDU.ics");
    let mut cal_file = File::open(&cal_path)?;
    let mut cal = read_calendar_from_file(cal_file);
    Ok(())
}
