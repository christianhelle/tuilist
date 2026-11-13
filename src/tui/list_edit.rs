//! Reading the dates and reminders typed into prompts and the task form.

use jiff::{
    Span, Timestamp, Zoned,
    civil::{Date, Weekday},
    tz::TimeZone,
};

const DATE_HINT: &str = "Use 2026-11-30, today, tomorrow, mon to sun or +3";
const REMINDER_HINT: &str = "Use 2026-11-30 09:00, or leave it empty";

/// Reads what was typed for a due date. Nothing clears it; `today`,
/// `tomorrow`, a weekday, `+3` and an ISO date are all understood.
pub fn parse_due(text: &str, today: Date) -> Result<Option<Date>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    match text.to_lowercase().as_str() {
        "today" => return Ok(Some(today)),
        "tomorrow" | "tmr" => {
            return Ok(Some(today.tomorrow().map_err(|_| DATE_HINT.to_owned())?));
        }
        "yesterday" => {
            return Ok(Some(today.yesterday().map_err(|_| DATE_HINT.to_owned())?));
        }
        _ => {}
    }
    if let Some(days) = text.strip_prefix('+')
        && let Ok(days) = days.trim().parse::<i64>()
    {
        return today
            .checked_add(Span::new().days(days))
            .map(Some)
            .map_err(|_| DATE_HINT.to_owned());
    }
    if let Some(weekday) = weekday(&text.to_lowercase()) {
        for ahead in 0..7 {
            let Some(date) = today.checked_add(Span::new().days(ahead)).ok() else {
                break;
            };
            if date.weekday() == weekday {
                return Ok(Some(date));
            }
        }
    }
    if let Ok(date) = text.parse::<Date>() {
        return Ok(Some(date));
    }
    Err(DATE_HINT.to_owned())
}

/// Reads what was typed for a reminder in the system's time zone. Nothing
/// clears it.
pub fn parse_reminder(text: &str, zone: &TimeZone) -> Result<Option<Timestamp>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let text = text.replace(' ', "T");
    let local = text
        .parse::<jiff::civil::DateTime>()
        .map_err(|_| REMINDER_HINT.to_owned())?;
    let zoned = local
        .to_zoned(zone.clone())
        .map_err(|_| REMINDER_HINT.to_owned())?;
    Ok(Some(zoned.timestamp()))
}

/// A reminder as the form shows it.
pub fn format_reminder(reminder: Timestamp, zone: &TimeZone) -> String {
    reminder
        .to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M")
        .to_string()
}

fn weekday(text: &str) -> Option<Weekday> {
    match text {
        "mon" | "monday" => Some(Weekday::Monday),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tuesday),
        "wed" | "wednesday" => Some(Weekday::Wednesday),
        "thu" | "thur" | "thurs" | "thursday" => Some(Weekday::Thursday),
        "fri" | "friday" => Some(Weekday::Friday),
        "sat" | "saturday" => Some(Weekday::Saturday),
        "sun" | "sunday" => Some(Weekday::Sunday),
        _ => None,
    }
}

/// The date and time a reminder is shown with, in `zone`.
pub fn format_datetime(at: Timestamp, zone: &TimeZone) -> String {
    at.to_zoned(zone.clone())
        .strftime("%a %-d %b %Y, %H:%M")
        .to_string()
}

/// A date as the details pane shows it.
pub fn format_date(date: Date) -> String {
    date.strftime("%a %-d %b %Y").to_string()
}

/// A timestamp's date as the details pane shows it.
pub fn format_timestamp_date(at: Timestamp, zone: &TimeZone) -> String {
    format_date(at.to_zoned(zone.clone()).date())
}

/// The time of a timestamp, in `zone`.
pub fn format_time(at: Timestamp, zone: &TimeZone) -> String {
    at.to_zoned(zone.clone()).strftime("%H:%M").to_string()
}

/// A local clock reading, for tests and demo data that must not change.
pub fn zoned(text: &str) -> Zoned {
    text.parse().expect("a valid zoned timestamp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn today() -> Date {
        date(2026, 11, 20)
    }

    #[test]
    fn nothing_clears_the_date() {
        assert_eq!(parse_due("", today()), Ok(None));
        assert_eq!(parse_due("   ", today()), Ok(None));
    }

    #[test]
    fn today_tomorrow_and_yesterday_are_understood() {
        assert_eq!(parse_due("today", today()), Ok(Some(today())));
        assert_eq!(parse_due("tomorrow", today()), Ok(Some(date(2026, 11, 21))));
        assert_eq!(
            parse_due("yesterday", today()),
            Ok(Some(date(2026, 11, 19)))
        );
    }

    #[test]
    fn plus_counts_days_from_today() {
        assert_eq!(parse_due("+3", today()), Ok(Some(date(2026, 11, 23))));
        assert_eq!(parse_due("+0", today()), Ok(Some(today())));
    }

    #[test]
    fn a_weekday_is_the_next_one_including_today() {
        assert_eq!(parse_due("fri", today()), Ok(Some(today())));
        assert_eq!(parse_due("mon", today()), Ok(Some(date(2026, 11, 23))));
        assert_eq!(parse_due("sun", today()), Ok(Some(date(2026, 11, 22))));
    }

    #[test]
    fn an_iso_date_is_read_as_written() {
        assert_eq!(
            parse_due("2026-12-01", today()),
            Ok(Some(date(2026, 12, 1)))
        );
    }

    #[test]
    fn anything_else_is_refused() {
        assert!(parse_due("next week", today()).is_err());
        assert!(parse_due("2026-13-01", today()).is_err());
    }

    #[test]
    fn a_reminder_is_a_local_date_and_time() {
        let zone = TimeZone::UTC;

        let reminder = parse_reminder("2026-11-30 09:00", &zone).unwrap().unwrap();

        assert_eq!(format_reminder(reminder, &zone), "2026-11-30 09:00");
        assert_eq!(format_datetime(reminder, &zone), "Mon 30 Nov 2026, 09:00");
    }

    #[test]
    fn nothing_clears_the_reminder() {
        assert_eq!(parse_reminder("", &TimeZone::UTC), Ok(None));
    }

    #[test]
    fn a_bad_reminder_is_refused() {
        assert!(parse_reminder("tomorrow", &TimeZone::UTC).is_err());
    }
}
