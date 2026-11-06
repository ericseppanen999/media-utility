use crate::domain::*;

pub fn timestamp(text: &str) -> Result<i64> {
    let parts: Vec<&str> = text.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return Err(AppError::new("TIMESTAMP", "Use M:SS or H:MM:SS"));
    }
    let mut total = 0i64;
    for (i, part) in parts.iter().enumerate() {
        let value: i64 = part.parse().map_err(|_| {
            AppError::new(
                "TIMESTAMP",
                "Timestamp must contain whole, non-negative numbers",
            )
        })?;
        if value < 0 || (i > 0 && (value >= 60 || part.len() != 2)) {
            return Err(AppError::new(
                "TIMESTAMP",
                "Minutes and seconds after a colon must be 00–59",
            ));
        }
        total = total
            .checked_mul(60)
            .and_then(|t| t.checked_add(value))
            .ok_or_else(|| AppError::new("TIMESTAMP", "Timestamp is too large"))?;
    }
    total
        .checked_mul(1_000_000)
        .ok_or_else(|| AppError::new("TIMESTAMP", "Timestamp is too large"))
}

pub fn parse(text: &str, duration: i64) -> Result<ParsedSegments> {
    if duration <= 0 {
        return Err(AppError::new(
            "DURATION",
            "A known source duration is required",
        ));
    }
    let mut segments: Vec<Segment> = Vec::new();
    for (line_no, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let fail = |message: String| {
            AppError::new("SEGMENT_LINE", format!("Line {}: {}", line_no + 1, message))
        };
        let (time, title) = if raw.starts_with('[') {
            let close = raw
                .find(']')
                .ok_or_else(|| fail("Unclosed timestamp link".into()))?;
            let rest = &raw[close + 1..];
            let end = rest
                .find(')')
                .ok_or_else(|| fail("Unclosed timestamp URL".into()))?;
            (&raw[1..close], rest[end + 1..].trim())
        } else {
            let split = raw
                .find(char::is_whitespace)
                .ok_or_else(|| fail("Add a title after the timestamp".into()))?;
            (&raw[..split], raw[split..].trim())
        };
        let start = if time.starts_with("https://") {
            let url = url::Url::parse(time).map_err(|_| fail("Invalid timestamp link".into()))?;
            let t = url
                .query_pairs()
                .find(|(k, _)| k == "t" || k == "start")
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| fail("Link has no t or start timestamp".into()))?;
            parse_link_time(&t).map_err(|e| fail(e.message))?
        } else {
            timestamp(time).map_err(|e| fail(e.message))?
        };
        let title = title.trim_start_matches(['-', '–', '—']).trim();
        if title.is_empty() {
            return Err(fail("Track title is empty".into()));
        }
        if start >= duration {
            return Err(fail("Start must be before the source ends".into()));
        }
        if let Some(prev) = segments.last_mut() {
            if start <= prev.start_us {
                return Err(fail(
                    "Starts must increase; duplicates and overlaps are not allowed".into(),
                ));
            }
            prev.end_us = start;
        }
        segments.push(Segment {
            id: id(),
            start_us: start,
            end_us: duration,
            title: title.into(),
        });
    }
    if segments.is_empty() {
        return Err(AppError::new(
            "SEGMENTS",
            "Enter at least one timestamp and title",
        ));
    }
    Ok(ParsedSegments {
        unused_intro_us: segments[0].start_us,
        segments,
    })
}

fn parse_link_time(s: &str) -> Result<i64> {
    if let Ok(n) = s.parse::<i64>() {
        return n
            .checked_mul(1_000_000)
            .filter(|n| *n >= 0)
            .ok_or_else(|| AppError::new("TIMESTAMP", "Invalid linked time"));
    }
    let mut digits = String::new();
    let mut seconds = 0i64;
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c)
        } else {
            let factor = match c {
                'h' => 3600,
                'm' => 60,
                's' => 1,
                _ => return Err(AppError::new("TIMESTAMP", "Invalid linked time")),
            };
            let n = digits
                .parse::<i64>()
                .map_err(|_| AppError::new("TIMESTAMP", "Invalid linked time"))?;
            seconds = seconds
                .checked_add(
                    n.checked_mul(factor)
                        .ok_or_else(|| AppError::new("TIMESTAMP", "Time too large"))?,
                )
                .ok_or_else(|| AppError::new("TIMESTAMP", "Time too large"))?;
            digits.clear();
        }
    }
    if !digits.is_empty() {
        return Err(AppError::new("TIMESTAMP", "Linked time requires a unit"));
    }
    seconds
        .checked_mul(1_000_000)
        .ok_or_else(|| AppError::new("TIMESTAMP", "Time too large"))
}

pub fn validate(segments: &[Segment], duration: i64, independent: bool) -> Result<()> {
    if segments.is_empty() || segments.len() > 1000 {
        return Err(AppError::new("SEGMENTS", "Choose 1–1000 regions"));
    }
    let mut previous_end = 0;
    let mut ids = std::collections::HashSet::new();
    for (i, s) in segments.iter().enumerate() {
        if !ids.insert(&s.id)
            || s.start_us < 0
            || s.end_us <= s.start_us
            || s.end_us > duration
            || s.title.trim().is_empty()
        {
            return Err(AppError::new(
                "SEGMENTS",
                format!(
                    "Region {} needs a unique ID, title, and valid source bounds",
                    i + 1
                ),
            ));
        }
        if !independent && s.start_us < previous_end {
            return Err(AppError::new(
                "OVERLAP",
                format!("Region {} overlaps the preceding region", i + 1),
            ));
        }
        previous_end = s.end_us;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_reports_intro() {
        let p = parse("0:12 Intro\n1:05 Main\n1:02:00 Finale", 4_000_000_000).unwrap();
        assert_eq!(p.unused_intro_us, 12_000_000);
        assert_eq!(p.segments[0].end_us, 65_000_000);
    }

    #[test]
    fn rejects_bad_lines() {
        for s in [
            "0:00 A\n0:00 B",
            "1:00 A\n0:30 B",
            "0:61 A",
            "0:00",
            "-1:00 A",
            "99:00 A",
        ] {
            assert!(parse(s, 120_000_000).is_err(), "{s}")
        }
    }

    #[test]
    fn links() {
        assert_eq!(
            parse(
                "[0:10](https://youtu.be/a?t=10) Song\nhttps://youtu.be/a?t=1m20s Next",
                100_000_000
            )
            .unwrap()
            .segments[1]
                .start_us,
            80_000_000
        );
    }

    #[test]
    fn overlap_policy() {
        let mut p = parse("0:00 A\n0:30 B", 60_000_000).unwrap();
        p.segments[0].end_us = 40_000_000;
        assert!(validate(&p.segments, 60_000_000, false).is_err());
        assert!(validate(&p.segments, 60_000_000, true).is_ok());
    }
}
