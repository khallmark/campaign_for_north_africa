//! Source-track labels only; not an executable calendar or reinforcement schedule.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub(super) fn label(turn: usize) -> String {
    assert!((1..=111).contains(&turn));
    let offset = turn + 1;
    let month = 8 + offset / 4;
    format!(
        "{} {}, {}",
        MONTHS[month % 12],
        ["I", "II", "III", "IV"][offset % 4],
        1940 + month / 12
    )
}

pub(super) fn panel(turn: usize, stage: usize) -> String {
    let weeks: String = (1..=111).map(|n| format!(r#"<button data-action="choose-turn" data-value="{n}" aria-pressed="{}" aria-label="Game-Turn {n}: {}"><b>{n}</b><span>{}</span></button>"#, n == turn, label(n), label(n))).collect();
    let stages: String = (1..=3).map(|n| format!(r#"<button data-action="choose-stage" data-value="{n}" aria-pressed="{}">Stage {n}</button>"#, n == stage)).collect();
    format!(
        r#"<h2>Campaign calendar</h2><p><strong>111 Game-Turns, not 111 days.</strong> Each Game-Turn represents approximately one week, divided into three Operations Stages of roughly 2–3 days (§5.1).</p><p>The printed track runs from September III, 1940 to January I, 1943. Its months use I–IV; these labels are not Gregorian seven-day increments.</p><div class="calendar-current"><strong id="calendar-selection" role="status">{} · Game-Turn {turn} · Operations Stage {stage}</strong><div class="calendar-stages" aria-label="Reference operations stage">{stages}</div></div><p class="reference-note">Reference selection only. Changing this cursor does not advance an operation, resolve phases or release reinforcements.</p><div class="calendar-weeks" aria-label="111 Game-Turn reference track">{weeks}</div><details class="calendar-notes"><summary>Sequence of play and source notes</summary><h3>Land game (§5.2)</h3><p>Initiative determination → Naval convoy scheduling and tactical shipping → Operations Stages 1, 2 and 3 → End of Game-Turn.</p><h3>Within each Operations Stage</h3><p>Initiative declaration; weather; organization (reorganization, construction, training); convoy arrivals; Commonwealth fleet; then reserve designation, movement and combat, truck convoys, rail movement, repairs and patrols for both players. Air and Logistics variants extend this sequence (§§33, 48); this reference does not execute any variant.</p><p>Labels follow the supplied VASSAL board’s printed Game-Turn track. Reinforcement abbreviations and track events have not been adjudicated into a complete schedule. Inspect the original track for that content; catalog candidates are not a reinforcement schedule.</p><button data-action="source-track">View original track on map</button></details>"#,
        label(turn)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_track_month_and_year_boundaries() {
        // Independently read anchors on the supplied printed track, not Gregorian arithmetic.
        for (turn, expected) in [
            (1, "September III, 1940"),
            (2, "September IV, 1940"),
            (3, "October I, 1940"),
            (14, "December IV, 1940"),
            (15, "January I, 1941"),
            (62, "December IV, 1941"),
            (63, "January I, 1942"),
            (110, "December IV, 1942"),
            (111, "January I, 1943"),
        ] {
            assert_eq!(label(turn), expected);
        }
        let html = panel(63, 2);
        assert_eq!(html.matches("data-action=\"choose-turn\"").count(), 111);
        assert!(html.contains("111 Game-Turns, not 111 days"));
    }
}
