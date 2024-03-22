use chrono::{DateTime, FixedOffset, Utc};

pub fn datetime_to_string(datetime: DateTime<Utc>) -> String {
    let offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC +8
    datetime
        .with_timezone(&offset)
        .format("%d/%m %H:%M")
        .to_string()
}
