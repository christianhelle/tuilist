//! The rows of the tasks pane and the dates shown after their titles.

use jiff::civil::Date;

/// How a due date reads beside a task: `Today`, `Fri 20`, or `20 Nov`.
pub fn due_text(due: Date, today: Date) -> String {
    if due == today {
        return "Today".to_owned();
    }
    if let Ok(tomorrow) = today.tomorrow()
        && due == tomorrow
    {
        return "Tomorrow".to_owned();
    }
    if let Ok(yesterday) = today.yesterday()
        && due == yesterday
    {
        return "Yesterday".to_owned();
    }
    let days = due.since(today).map(|span| span.get_days()).unwrap_or(0);
    if (1..=6).contains(&days) {
        return due.strftime("%a %-d").to_string();
    }
    if due.year() == today.year() {
        due.strftime("%-d %b").to_string()
    } else {
        due.strftime("%-d %b %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn near_dates_say_today_tomorrow_or_yesterday() {
        let today = date(2026, 11, 20);

        assert_eq!(due_text(today, today), "Today");
        assert_eq!(due_text(date(2026, 11, 21), today), "Tomorrow");
        assert_eq!(due_text(date(2026, 11, 19), today), "Yesterday");
    }

    #[test]
    fn dates_within_a_week_name_the_day_and_number() {
        let today = date(2026, 11, 20);

        assert_eq!(due_text(date(2026, 11, 23), today), "Mon 23");
        assert_eq!(due_text(date(2026, 11, 26), today), "Thu 26");
    }

    #[test]
    fn later_dates_name_the_day_and_month_and_the_year_when_it_differs() {
        let today = date(2026, 11, 20);

        assert_eq!(due_text(date(2026, 12, 1), today), "1 Dec");
        assert_eq!(due_text(date(2027, 1, 4), today), "4 Jan 2027");
    }
}
