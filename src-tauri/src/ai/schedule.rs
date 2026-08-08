use chrono::{DateTime, Local, Timelike};

pub fn is_due(now: DateTime<Local>, org_time: &str, last_org_date: Option<&str>) -> bool {
    let today = now.format("%Y-%m-%d").to_string();
    if last_org_date == Some(today.as_str()) {
        return false;
    }
    let Some((h, m)) = parse_hhmm(org_time) else {
        return false;
    };
    (now.hour() as u32, now.minute() as u32) >= (h, m)
}

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    Some((h.parse().ok()?, m.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn due_when_past_org_time_and_not_ran_today() {
        let now = Local::now().with_hour(23).unwrap().with_minute(0).unwrap();
        assert!(is_due(now, "22:00", None));
        let today = now.format("%Y-%m-%d").to_string();
        assert!(!is_due(now, "22:00", Some(today.as_str())));
    }

    #[test]
    fn not_due_before_org_time() {
        let now = Local::now().with_hour(21).unwrap().with_minute(0).unwrap();
        assert!(!is_due(now, "22:00", None));
    }
}
