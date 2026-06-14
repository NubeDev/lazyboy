use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::Space;
use crate::Id;

/// The `trigger_config_json` a schedule-triggered workflow carries: a
/// standard 5-field cron expression in UTC plus the space the run lands
/// in (SCOPE.md "Workflows and automation", schedule trigger).
/// Deserialized by the schedule tick to decide whether the workflow is
/// due and where to fire it; the tick itself (a host-side clock) is out
/// of the mobile-safe crate graph, but the *decision* of whether a
/// minute matches is pure and lives here so it is testable without a
/// clock.
///
/// A workflow is workspace-scoped (the `workflows` row), but a run needs
/// a space. A feed trigger gets its space from the inbound event; a
/// schedule has no event, so the target space is named here in the
/// trigger config. This keeps the workflow table unchanged and the
/// space binding explicit and auditable.
///
/// Cron fields are `minute hour day-of-month month day-of-week`, each
/// either `*` or a comma list of integers (e.g. `0 9 * * 1` = 09:00 UTC
/// every Monday). Ranges and steps are deliberately not supported: the
/// MVP trigger needs fixed instants, and a comma list covers them
/// without a dependency on a full cron crate (which would pull chrono
/// into the mobile-safe graph). Day-of-week is `0..=6`, Sunday = 0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleTrigger {
    pub cron: String,
    pub space_id: Id<Space>,
}

/// A `cron` string that does not parse into five `*`-or-int-list fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadCron(pub String);

impl std::fmt::Display for BadCron {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid cron expression: {}", self.0)
    }
}
impl std::error::Error for BadCron {}

impl ScheduleTrigger {
    /// Whether the schedule fires at some minute in the half-open window
    /// `(since, at]`. The window form is load-bearing: the tick polls on
    /// an interval, so it asks "did any matching minute pass since I last
    /// looked", which fires each match exactly once even if a poll is
    /// late or the interval spans several matching minutes. `since` is
    /// the previous tick instant (exclusive); `at` is now (inclusive).
    /// A `since >= at` window matches nothing.
    pub fn fires_between(
        &self,
        since: OffsetDateTime,
        at: OffsetDateTime,
    ) -> Result<bool, BadCron> {
        let schedule = Cron::parse(&self.cron)?;
        if since >= at {
            return Ok(false);
        }
        // Step minute by minute through the window. The first candidate
        // is the minute strictly after `since` (truncated to the minute),
        // so a match exactly on `since` does not re-fire; the last is the
        // minute of `at` inclusive. The window the tick passes is one
        // poll interval (seconds to minutes), so this loop is short.
        let mut minute = floor_minute(since) + time::Duration::minutes(1);
        while minute <= at {
            if schedule.matches(minute) {
                return Ok(true);
            }
            minute += time::Duration::minutes(1);
        }
        Ok(false)
    }
}

fn floor_minute(t: OffsetDateTime) -> OffsetDateTime {
    t.replace_second(0)
        .and_then(|t| t.replace_nanosecond(0))
        .unwrap_or(t)
}

/// One parsed cron field: any value, or an explicit set of allowed ones.
#[derive(Debug, PartialEq, Eq)]
enum Field {
    Any,
    Only(Vec<u8>),
}

impl Field {
    fn parse(token: &str, min: u8, max: u8) -> Result<Self, ()> {
        if token == "*" {
            return Ok(Field::Any);
        }
        let mut values = Vec::new();
        for part in token.split(',') {
            let n: u8 = part.parse().map_err(|_| ())?;
            if n < min || n > max {
                return Err(());
            }
            values.push(n);
        }
        if values.is_empty() {
            return Err(());
        }
        Ok(Field::Only(values))
    }

    fn allows(&self, value: u8) -> bool {
        match self {
            Field::Any => true,
            Field::Only(set) => set.contains(&value),
        }
    }
}

/// A parsed 5-field cron expression evaluated in UTC.
struct Cron {
    minute: Field,
    hour: Field,
    day_of_month: Field,
    month: Field,
    day_of_week: Field,
}

impl Cron {
    fn parse(expr: &str) -> Result<Self, BadCron> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(BadCron(expr.to_owned()));
        }
        let bad = || BadCron(expr.to_owned());
        Ok(Cron {
            minute: Field::parse(fields[0], 0, 59).map_err(|_| bad())?,
            hour: Field::parse(fields[1], 0, 23).map_err(|_| bad())?,
            day_of_month: Field::parse(fields[2], 1, 31).map_err(|_| bad())?,
            month: Field::parse(fields[3], 1, 12).map_err(|_| bad())?,
            day_of_week: Field::parse(fields[4], 0, 6).map_err(|_| bad())?,
        })
    }

    /// Standard cron day semantics: when both day-of-month and
    /// day-of-week are restricted, the entry fires if *either* matches;
    /// when only one is restricted, that one must match. A `*` on a day
    /// field means "no restriction from this field".
    fn matches(&self, t: OffsetDateTime) -> bool {
        let dom_restricted = !matches!(self.day_of_month, Field::Any);
        let dow_restricted = !matches!(self.day_of_week, Field::Any);
        // time's Weekday: Sunday::number_days_from_sunday() == 0.
        let dow = t.weekday().number_days_from_sunday();
        let day_ok = match (dom_restricted, dow_restricted) {
            (true, true) => {
                self.day_of_month.allows(t.day()) || self.day_of_week.allows(dow)
            }
            (true, false) => self.day_of_month.allows(t.day()),
            (false, true) => self.day_of_week.allows(dow),
            (false, false) => true,
        };
        self.minute.allows(t.minute())
            && self.hour.allows(t.hour())
            && self.month.allows(t.month() as u8)
            && day_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn trig(cron: &str) -> ScheduleTrigger {
        ScheduleTrigger {
            cron: cron.to_owned(),
            space_id: Id::new(),
        }
    }

    #[test]
    fn deserializes_cron_and_space_from_trigger_config_json() {
        let json = r#"{"cron":"0 9 * * 1","space_id":"6f1c8b1e-3c2a-4d5e-8f90-112233445566"}"#;
        let t: ScheduleTrigger = serde_json::from_str(json).unwrap();
        assert_eq!(t.cron, "0 9 * * 1");
        assert_eq!(
            t.space_id.to_string(),
            "6f1c8b1e-3c2a-4d5e-8f90-112233445566"
        );
    }

    #[test]
    fn rejects_wrong_field_count_and_out_of_range() {
        assert!(trig("* * * *")
            .fires_between(datetime!(2026-06-14 00:00 UTC), datetime!(2026-06-14 01:00 UTC))
            .is_err());
        assert!(trig("99 * * * *")
            .fires_between(datetime!(2026-06-14 00:00 UTC), datetime!(2026-06-14 01:00 UTC))
            .is_err());
    }

    #[test]
    fn every_minute_fires_for_any_nonempty_window() {
        let t = trig("* * * * *");
        assert!(t
            .fires_between(datetime!(2026-06-14 09:00 UTC), datetime!(2026-06-14 09:01 UTC))
            .unwrap());
    }

    #[test]
    fn empty_window_never_fires() {
        let t = trig("* * * * *");
        let at = datetime!(2026-06-14 09:00 UTC);
        assert!(!t.fires_between(at, at).unwrap());
    }

    #[test]
    fn fixed_minute_fires_once_when_crossed() {
        // 09:00 UTC daily. A window that steps across 09:00 fires.
        let t = trig("0 9 * * *");
        assert!(t
            .fires_between(datetime!(2026-06-14 08:59 UTC), datetime!(2026-06-14 09:00 UTC))
            .unwrap());
        // A window strictly before it does not.
        assert!(!t
            .fires_between(datetime!(2026-06-14 08:57 UTC), datetime!(2026-06-14 08:59 UTC))
            .unwrap());
    }

    #[test]
    fn match_exactly_on_since_does_not_refire() {
        // since is exclusive: a poll whose previous instant was 09:00
        // must not re-fire the 09:00 entry.
        let t = trig("0 9 * * *");
        assert!(!t
            .fires_between(datetime!(2026-06-14 09:00 UTC), datetime!(2026-06-14 09:00:30 UTC))
            .unwrap());
    }

    #[test]
    fn day_of_week_monday_only() {
        // 2026-06-15 is a Monday; 2026-06-14 is a Sunday.
        let t = trig("0 9 * * 1");
        assert!(t
            .fires_between(datetime!(2026-06-15 08:59 UTC), datetime!(2026-06-15 09:00 UTC))
            .unwrap());
        assert!(!t
            .fires_between(datetime!(2026-06-14 08:59 UTC), datetime!(2026-06-14 09:00 UTC))
            .unwrap());
    }

    #[test]
    fn comma_list_of_minutes() {
        let t = trig("0,30 * * * *");
        assert!(t
            .fires_between(datetime!(2026-06-14 09:29 UTC), datetime!(2026-06-14 09:30 UTC))
            .unwrap());
        assert!(!t
            .fires_between(datetime!(2026-06-14 09:31 UTC), datetime!(2026-06-14 09:44 UTC))
            .unwrap());
    }

    #[test]
    fn dom_or_dow_both_restricted_is_a_union() {
        // Fires on the 1st of the month OR any Monday (standard cron).
        let t = trig("0 0 1 * 1");
        // 2026-07-01 is a Wednesday: matches via day-of-month.
        assert!(t
            .fires_between(datetime!(2026-06-30 23:59 UTC), datetime!(2026-07-01 00:00 UTC))
            .unwrap());
        // 2026-06-15 is a Monday, not the 1st: matches via day-of-week.
        assert!(t
            .fires_between(datetime!(2026-06-14 23:59 UTC), datetime!(2026-06-15 00:00 UTC))
            .unwrap());
        // 2026-06-16 is a Tuesday, not the 1st: no match.
        assert!(!t
            .fires_between(datetime!(2026-06-15 23:59 UTC), datetime!(2026-06-16 00:00 UTC))
            .unwrap());
    }

    #[test]
    fn wide_window_spanning_multiple_days_still_fires() {
        // A late tick (missed a day) still fires a daily entry exactly by
        // detecting the crossed minute in the window.
        let t = trig("0 9 * * *");
        assert!(t
            .fires_between(datetime!(2026-06-13 12:00 UTC), datetime!(2026-06-14 12:00 UTC))
            .unwrap());
    }
}
