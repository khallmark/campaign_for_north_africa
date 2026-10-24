//! Presentation-only movement. Coordinates and headings come from the renderer,
//! never feed back into simulation, and use the requestAnimationFrame clock.

const MIN_DURATION_MS: f64 = 420.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    /// World bearing clockwise from east; may be outside 0..360 for short turns.
    /// Select directional artwork for this bearing; never rotate the image itself.
    pub heading: f64,
}

/// Select the nearest upright yaw view from a clockwise, evenly spaced atlas.
/// Frame zero faces east; ties select the clockwise frame. `frame_count` must
/// be nonzero. Unwrapped animation headings are normalized only for this lookup.
pub fn directional_frame(heading: f64, frame_count: usize) -> usize {
    (heading.rem_euclid(360.0) * frame_count as f64 / 360.0).round() as usize % frame_count
}

/// One accepted adjacent-hex step. Centers and the shared-edge normal bearing
/// come from map geometry; playback never derives a bearing from pixel deltas.
#[derive(Clone, Copy, Debug)]
pub struct RouteLeg {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub bearing: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaybackState {
    Turning,
    Translating,
    Settled,
}

/// A timeline of turn-then-translate legs, independent of DOM and frame cadence.
pub struct RoutePlayback {
    legs: Vec<TimedLeg>,
    target: Pose,
    settled: bool,
}

impl RoutePlayback {
    /// Legs must be contiguous, starting at start_pose's center. The caller owns
    /// route validation; this model only schedules the supplied geometry.
    pub fn new(start_pose: Pose, legs: Vec<RouteLeg>, now_ms: f64) -> Self {
        let mut target = start_pose;
        let mut started_ms = now_ms;
        let legs = legs
            .into_iter()
            .map(|leg| {
                let from = Pose {
                    x: leg.from.0,
                    y: leg.from.1,
                    heading: target.heading,
                };
                target = Pose {
                    x: leg.to.0,
                    y: leg.to.1,
                    heading: from.heading + shortest_turn(from.heading, leg.bearing),
                };
                let timed = TimedLeg::new(from, target, started_ms);
                started_ms = timed.end_ms();
                timed
            })
            .collect();
        Self {
            legs,
            target,
            settled: false,
        }
    }

    /// Returns the visible pose, zero-based leg index, and phase. At a shared
    /// center the next leg starts; settled routes retain their last leg index,
    /// while empty routes have none. Reduced motion permanently finishes the
    /// whole route, so disabling it cannot replay skipped legs.
    pub fn sample(
        &mut self,
        now_ms: f64,
        reduced_motion: bool,
    ) -> (Pose, Option<usize>, PlaybackState) {
        if !reduced_motion && !self.settled {
            for (index, leg) in self.legs.iter().enumerate() {
                if now_ms < leg.end_ms() {
                    let state = if now_ms < leg.started_ms + leg.turn_duration_ms {
                        PlaybackState::Turning
                    } else {
                        PlaybackState::Translating
                    };
                    return (leg.sample(now_ms), Some(index), state);
                }
            }
        }
        self.settled = true;
        (
            self.target,
            self.legs.len().checked_sub(1),
            PlaybackState::Settled,
        )
    }

    pub fn is_active(&self, now_ms: f64) -> bool {
        !self.settled && self.legs.last().is_some_and(|leg| now_ms < leg.end_ms())
    }
}

struct TimedLeg {
    from: Pose,
    target: Pose,
    started_ms: f64,
    turn_duration_ms: f64,
    travel_duration_ms: f64,
}

impl TimedLeg {
    fn new(from: Pose, target: Pose, started_ms: f64) -> Self {
        // Complete the turn in place before travelling. Each phase keeps its own
        // pacing; an already aligned unit starts moving without a turn delay.
        let distance = (target.x - from.x).hypot(target.y - from.y);
        let angle = (target.heading - from.heading).abs();
        let turn_duration_ms = if angle > 0.0 {
            (angle * 2.0).max(80.0)
        } else {
            0.0
        };
        let travel_duration_ms = if distance > 0.0 {
            (distance / 0.36).clamp(MIN_DURATION_MS, 1800.0)
        } else {
            0.0
        };
        Self {
            from,
            target,
            started_ms,
            turn_duration_ms,
            travel_duration_ms,
        }
    }

    fn sample(&self, now_ms: f64) -> Pose {
        let elapsed = now_ms - self.started_ms;
        let progress = |time: f64, duration: f64| {
            if duration == 0.0 {
                1.0
            } else {
                (time / duration).clamp(0.0, 1.0)
            }
        };
        // Constant angular speed gives discrete views an even cadence rather
        // than lingering on the first/last view and rushing through the middle.
        let turn = progress(elapsed, self.turn_duration_ms);
        let travel = progress(elapsed - self.turn_duration_ms, self.travel_duration_ms);
        let travel = travel * travel * (3.0 - 2.0 * travel);
        Pose {
            x: self.from.x + (self.target.x - self.from.x) * travel,
            y: self.from.y + (self.target.y - self.from.y) * travel,
            heading: self.from.heading + (self.target.heading - self.from.heading) * turn,
        }
    }

    fn end_ms(&self) -> f64 {
        self.started_ms + self.turn_duration_ms + self.travel_duration_ms
    }
}

/// Compatibility API for the sandbox's single-destination orders.
pub struct Motion {
    playback: RoutePlayback,
}

impl Motion {
    /// New units appear at their destination without animating from the origin.
    pub fn new(pose: Pose) -> Self {
        Self {
            playback: RoutePlayback::new(pose, vec![], 0.0),
        }
    }

    /// Change destination without jumping back when an order interrupts a turn.
    /// Repeated renders of an unchanged destination do not restart the clock.
    pub fn retarget(&mut self, pose: Pose, now_ms: f64) {
        let target = self.playback.target;
        if pose.x == target.x
            && pose.y == target.y
            && shortest_turn(target.heading, pose.heading).abs() < 1e-9
        {
            return;
        }
        let from = self.sample(now_ms, false);
        self.playback = RoutePlayback::new(
            from,
            vec![RouteLeg {
                from: (from.x, from.y),
                to: (pose.x, pose.y),
                bearing: pose.heading,
            }],
            now_ms,
        );
    }

    pub fn sample(&mut self, now_ms: f64, reduced_motion: bool) -> Pose {
        self.playback.sample(now_ms, reduced_motion).0
    }

    pub fn is_active(&self, now_ms: f64) -> bool {
        self.playback.is_active(now_ms)
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
    fn directional_frames_select_nearest_view_and_wrap_in_both_atlas_sizes() {
        for (count, step) in [(18, 20.0), (36, 10.0)] {
            for frame in 0..count {
                let bearing = frame as f64 * step;
                assert_eq!(directional_frame(bearing, count), frame);
                assert_eq!(directional_frame(bearing - 720.0, count), frame);
                assert_eq!(directional_frame(bearing + 720.0, count), frame);
                assert_eq!(directional_frame(bearing + step / 2.0 - 0.01, count), frame);
                assert_eq!(
                    directional_frame(bearing + step / 2.0, count),
                    (frame + 1) % count
                );
            }
            assert_eq!(directional_frame(-step / 2.0 - 0.01, count), count - 1);
            assert_eq!(directional_frame(-step / 2.0, count), 0);
            assert_eq!(directional_frame(360.0, count), 0);
        }
    }

    #[test]
    fn seam_turns_advance_through_adjacent_frames_not_a_full_sprite_cycle() {
        let mut motion = Motion::new(pose(0.0, 0.0, 340.0));
        motion.retarget(pose(72.0, 0.0, 20.0), 0.0);
        let frames = [0.0, 20.0, 40.0, 60.0, 80.0]
            .map(|time| directional_frame(motion.sample(time, false).heading, 18));
        assert_eq!(frames, [17, 0, 0, 1, 1]);
        motion.retarget(pose(0.0, 0.0, 340.0), 80.0);
        let reverse = [80.0, 100.0, 120.0, 140.0, 160.0]
            .map(|time| directional_frame(motion.sample(time, false).heading, 18));
        assert_eq!(reverse, [1, 1, 0, 0, 17]);
    }

    #[test]
    fn crosses_angle_seam_by_the_short_route_in_both_directions() {
        for (start, target, middle, end, duration) in [
            (350.0, 10.0, 360.0, 370.0, 80.0),
            (10.0, 350.0, 0.0, -10.0, 80.0),
            (170.0, -170.0, 180.0, 190.0, 80.0),
            (-170.0, 170.0, -180.0, -190.0, 80.0),
            (20.0, 200.0, -70.0, -160.0, 360.0),
        ] {
            let mut motion = Motion::new(pose(50.0, 80.0, start));
            motion.retarget(pose(50.0, 80.0, target), 100.0);
            assert_eq!(
                motion.sample(100.0 + duration / 2.0, false),
                pose(50.0, 80.0, middle)
            );
            assert_eq!(
                motion.sample(100.0 + duration, false),
                pose(50.0, 80.0, end)
            );
            assert!(!motion.is_active(100.0 + duration));
        }
    }

    #[test]
    fn completes_turn_in_place_before_moving_with_fixed_heading() {
        let start = pose(44.0, 42.0, 0.0);
        let target = pose(116.0, 166.0, 90.0);
        let mut motion = Motion::new(start);
        assert_eq!(motion.sample(10.0, false), start);
        assert!(!motion.is_active(10.0));
        motion.retarget(target, 100.0);
        assert_eq!(motion.sample(100.0, false), start);
        // Even turning cadence, then eased translation: 1/4 travel is 5/32.
        assert_eq!(motion.sample(145.0, false), pose(44.0, 42.0, 22.5));
        assert_eq!(motion.sample(235.0, false), pose(44.0, 42.0, 67.5));
        let before_move = motion.sample(279.9, false);
        assert_eq!((before_move.x, before_move.y), (44.0, 42.0));
        assert!(before_move.heading < 90.0);
        assert_eq!(motion.sample(280.0, false), pose(44.0, 42.0, 90.0));
        assert_eq!(motion.sample(385.0, false), pose(55.25, 61.375, 90.0));
        assert!(motion.is_active(699.9));
        assert_eq!(motion.sample(700.0, false), target);
        assert!(!motion.is_active(700.0));
        assert_eq!(motion.sample(10_000.0, false), target);
    }

    #[test]
    fn interrupted_orders_start_from_the_visible_pose() {
        let mut motion = Motion::new(pose(0.0, 40.0, 350.0));
        motion.retarget(pose(100.0, 80.0, 10.0), 0.0);
        let visible = pose(0.0, 40.0, 360.0);
        assert_eq!(motion.sample(40.0, false), visible);
        motion.retarget(pose(10.0, 100.0, 330.0), 40.0);
        assert_eq!(motion.sample(40.0, false), visible);
        assert_eq!(motion.sample(80.0, false), pose(0.0, 40.0, 345.0));
        assert_eq!(motion.sample(120.0, false), pose(0.0, 40.0, 330.0));
        // Interrupt travel as well: the new turn holds the visible position.
        let travelling = pose(5.0, 70.0, 330.0);
        assert_eq!(motion.sample(330.0, false), travelling);
        motion.retarget(pose(80.0, 10.0, 30.0), 330.0);
        assert_eq!(motion.sample(330.0, false), travelling);
        assert_eq!(motion.sample(390.0, false), pose(5.0, 70.0, 360.0));
        assert_eq!(motion.sample(450.0, false), pose(5.0, 70.0, 390.0));
        assert_eq!(motion.sample(870.0, false), pose(80.0, 10.0, 390.0));
    }

    #[test]
    fn repeated_renders_do_not_restart_an_order_or_spin_equivalent_angles() {
        let mut motion = Motion::new(pose(0.0, 0.0, 350.0));
        motion.retarget(pose(72.0, 0.0, 10.0), 0.0);
        motion.retarget(pose(72.0, 0.0, 370.0), 20.0);
        assert_eq!(motion.sample(40.0, false), pose(0.0, 0.0, 360.0));
        assert_eq!(motion.sample(80.0, false), pose(0.0, 0.0, 370.0));
        motion.retarget(pose(72.0, 0.0, -350.0), 120.0);
        assert_eq!(motion.sample(290.0, false), pose(36.0, 0.0, 370.0));
        assert_eq!(motion.sample(500.0, false), pose(72.0, 0.0, 370.0));
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

    #[test]
    fn aligned_units_move_immediately_and_long_moves_get_more_time() {
        let mut motion = Motion::new(pose(0.0, 0.0, 0.0));
        motion.retarget(pose(360.0, 0.0, 0.0), 0.0);
        assert_eq!(motion.sample(500.0, false), pose(180.0, 0.0, 0.0));
        assert!(motion.is_active(999.0));
        assert_eq!(motion.sample(1000.0, false), pose(360.0, 0.0, 0.0));
    }

    #[test]
    fn route_visits_each_center_and_finishes_each_turn_before_translation() {
        let legs = vec![
            RouteLeg {
                from: (10.0, 20.0),
                to: (82.0, 20.0),
                bearing: 0.0,
            },
            RouteLeg {
                from: (82.0, 20.0),
                to: (118.0, 82.0),
                bearing: 60.0,
            },
            RouteLeg {
                from: (118.0, 82.0),
                to: (46.0, 82.0),
                bearing: 180.0,
            },
        ];
        let mut route = RoutePlayback::new(pose(10.0, 20.0, -90.0), legs.clone(), 100.0);
        // Independent schedule: turns of 180, 120, 240 ms; each travel is 420 ms.
        // The second bearing is supplied by geometry, not atan2(62, 36).
        for (index, (start, turn_end, end)) in [
            (100.0, 280.0, 700.0),
            (700.0, 820.0, 1240.0),
            (1240.0, 1480.0, 1900.0),
        ]
        .into_iter()
        .enumerate()
        {
            let leg = legs[index];
            for time in [
                start,
                start + 1.0,
                (start + turn_end) / 2.0,
                turn_end - 0.01,
            ] {
                let (visible, current, state) = route.sample(time, false);
                assert_eq!(current, Some(index));
                assert_eq!(state, PlaybackState::Turning);
                assert_eq!((visible.x, visible.y), leg.from);
                assert_ne!(visible.heading, leg.bearing);
            }
            for time in [turn_end, turn_end + 105.0, end - 0.01] {
                let (visible, current, state) = route.sample(time, false);
                assert_eq!(current, Some(index));
                assert_eq!(state, PlaybackState::Translating);
                assert_eq!(visible.heading, leg.bearing);
            }
            let (center, _, _) = route.sample(end, false);
            assert_eq!((center.x, center.y), leg.to);
        }
        assert_eq!(
            route.sample(1900.0, false),
            (pose(46.0, 82.0, 180.0), Some(2), PlaybackState::Settled)
        );
        // A delayed frame uses the original schedule, not the time it was sampled.
        let mut skipped = RoutePlayback::new(pose(10.0, 20.0, -90.0), legs, 100.0);
        assert_eq!(
            skipped.sample(1585.0, false),
            (
                pose(106.75, 82.0, 180.0),
                Some(2),
                PlaybackState::Translating
            )
        );
    }

    #[test]
    fn empty_route_settles_and_reduced_motion_finishes_the_whole_route() {
        let start = pose(12.0, 34.0, 350.0);
        let mut empty = RoutePlayback::new(start, vec![], 100.0);
        assert_eq!(
            empty.sample(100.0, false),
            (start, None, PlaybackState::Settled)
        );
        let mut route = RoutePlayback::new(
            start,
            vec![
                RouteLeg {
                    from: (12.0, 34.0),
                    to: (84.0, 34.0),
                    bearing: 10.0,
                },
                RouteLeg {
                    from: (84.0, 34.0),
                    to: (120.0, 96.0),
                    bearing: 60.0,
                },
            ],
            100.0,
        );
        let final_sample = (pose(120.0, 96.0, 420.0), Some(1), PlaybackState::Settled);
        assert_eq!(route.sample(140.0, true), final_sample);
        assert_eq!(route.sample(141.0, false), final_sample);
    }
}
