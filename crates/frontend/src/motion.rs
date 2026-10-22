//! Presentation-only movement. Coordinates and headings come from the renderer,
//! never feed back into simulation, and use the requestAnimationFrame clock.

const DURATION_MS: f64 = 420.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    /// SVG rotation in degrees clockwise; may be outside 0..360 for short turns.
    pub heading: f64,
}

pub struct Motion {
    from: Pose,
    target: Pose,
    started_ms: f64,
    animating: bool,
}

impl Motion {
    /// New units appear at their destination without animating from the origin.
    pub fn new(pose: Pose) -> Self {
        Self {
            from: pose,
            target: pose,
            started_ms: 0.0,
            animating: false,
        }
    }

    /// Change destination without jumping back when an order interrupts a turn.
    /// Repeated renders of an unchanged destination do not restart the clock.
    pub fn retarget(&mut self, pose: Pose, now_ms: f64) {
        if pose.x == self.target.x
            && pose.y == self.target.y
            && shortest_turn(self.target.heading, pose.heading).abs() < 1e-9
        {
            return;
        }
        self.from = self.sample(now_ms, false);
        self.target = Pose {
            heading: self.from.heading + shortest_turn(self.from.heading, pose.heading),
            ..pose
        };
        self.started_ms = now_ms;
        self.animating = self.from != self.target;
    }

    /// Reduced motion finishes the current order permanently, including when the
    /// preference changes mid-animation. Turning it off never replays old motion.
    pub fn sample(&mut self, now_ms: f64, reduced_motion: bool) -> Pose {
        if reduced_motion || !self.is_active(now_ms) {
            self.animating = false;
            return self.target;
        }
        let progress = ((now_ms - self.started_ms) / DURATION_MS).clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        Pose {
            x: self.from.x + (self.target.x - self.from.x) * eased,
            y: self.from.y + (self.target.y - self.from.y) * eased,
            heading: self.from.heading + (self.target.heading - self.from.heading) * eased,
        }
    }

    pub fn is_active(&self, now_ms: f64) -> bool {
        self.animating && now_ms < self.started_ms + DURATION_MS
    }
}

fn shortest_turn(from: f64, to: f64) -> f64 {
    // Exactly opposite headings consistently turn counterclockwise.
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(x: f64, y: f64, heading: f64) -> Pose {
        Pose { x, y, heading }
    }

    #[test]
    fn crosses_angle_seam_by_the_short_route_in_both_directions() {
        for (start, target, middle, end) in [
            (350.0, 10.0, 360.0, 370.0),
            (10.0, 350.0, 0.0, -10.0),
            (170.0, -170.0, 180.0, 190.0),
            (-170.0, 170.0, -180.0, -190.0),
            (20.0, 200.0, -70.0, -160.0),
        ] {
            let mut motion = Motion::new(pose(50.0, 80.0, start));
            motion.retarget(pose(50.0, 80.0, target), 100.0);
            assert_eq!(motion.sample(310.0, false), pose(50.0, 80.0, middle));
            assert_eq!(motion.sample(520.0, false), pose(50.0, 80.0, end));
            assert!(!motion.is_active(520.0));
        }
    }

    #[test]
    fn moves_and_turns_together_with_eased_endpoints() {
        let start = pose(44.0, 42.0, 0.0);
        let target = pose(116.0, 166.0, 90.0);
        let mut motion = Motion::new(start);
        assert_eq!(motion.sample(10.0, false), start);
        assert!(!motion.is_active(10.0));
        motion.retarget(target, 100.0);
        assert_eq!(motion.sample(100.0, false), start);
        // At 1/4 time smoothstep is 5/32, not a linear 1/4.
        assert_eq!(motion.sample(205.0, false), pose(55.25, 61.375, 14.0625));
        assert!(motion.is_active(519.9));
        assert_eq!(motion.sample(520.0, false), target);
        assert!(!motion.is_active(520.0));
        assert_eq!(motion.sample(10_000.0, false), target);
    }

    #[test]
    fn interrupted_orders_start_from_the_visible_pose() {
        let mut motion = Motion::new(pose(0.0, 40.0, 350.0));
        motion.retarget(pose(100.0, 80.0, 10.0), 0.0);
        let visible = pose(50.0, 60.0, 360.0);
        assert_eq!(motion.sample(210.0, false), visible);
        motion.retarget(pose(10.0, 100.0, 330.0), 210.0);
        assert_eq!(motion.sample(210.0, false), visible);
        assert_eq!(motion.sample(420.0, false), pose(30.0, 80.0, 345.0));
        assert_eq!(motion.sample(630.0, false), pose(10.0, 100.0, 330.0));
    }

    #[test]
    fn repeated_renders_do_not_restart_an_order_or_spin_equivalent_angles() {
        let mut motion = Motion::new(pose(0.0, 0.0, 350.0));
        motion.retarget(pose(72.0, 0.0, 10.0), 0.0);
        motion.retarget(pose(72.0, 0.0, 370.0), 105.0);
        assert_eq!(motion.sample(210.0, false), pose(36.0, 0.0, 360.0));
        assert_eq!(motion.sample(420.0, false), pose(72.0, 0.0, 370.0));
        motion.retarget(pose(72.0, 0.0, -350.0), 500.0);
        assert!(!motion.is_active(500.0));
    }

    #[test]
    fn reduced_motion_snaps_and_cannot_replay_when_preference_changes() {
        let mut motion = Motion::new(pose(12.0, 34.0, 10.0));
        motion.retarget(pose(84.0, 96.0, 350.0), 100.0);
        assert_eq!(motion.sample(200.0, true), pose(84.0, 96.0, -10.0));
        assert!(!motion.is_active(200.0));
        assert_eq!(motion.sample(201.0, false), pose(84.0, 96.0, -10.0));
        motion.retarget(pose(156.0, 158.0, 20.0), 300.0);
        assert_eq!(motion.sample(300.0, true), pose(156.0, 158.0, 20.0));
        assert!(!motion.is_active(300.0));
    }
}
