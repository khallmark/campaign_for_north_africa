use cna_map::geometry::BoardPoint;
use cna_map::viewport::{ScreenPoint, Viewport};

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn viewport_css_transform_roundtrips_at_required_sizes_zooms_and_dprs() {
    for css_size in [[2458.0, 1358.0], [1280.0, 800.0]] {
        for zoom in [0.05, 1.0, 4.0] {
            for dpr in [1.0, 2.0] {
                let view = Viewport::new(
                    BoardPoint {
                        x: 4321.25,
                        y: 1777.75,
                    },
                    zoom,
                    css_size,
                    dpr,
                )
                .unwrap();
                for (x, y) in [
                    (0.0, 0.0),
                    (14310.0, 4632.0),
                    (7213.625, 1515.95),
                    (-0.25, 4632.5),
                ] {
                    let p = BoardPoint { x, y };
                    let projected = view.to_screen(p);
                    let roundtrip = view.to_board(projected);
                    close(roundtrip.x, x);
                    close(roundtrip.y, y);
                    assert_eq!(
                        projected,
                        Viewport::new(view.center(), zoom, css_size, 1.0)
                            .unwrap()
                            .to_screen(p)
                    );
                }
                assert_eq!(view.device_pixel_ratio(), dpr);
            }
        }
    }
    let view = Viewport::new(BoardPoint { x: 200.0, y: 300.0 }, 4.0, [1280.0, 800.0], 2.0).unwrap();
    // Independent asymmetric value: catches inverses that consistently use DPR or wrong units.
    assert_eq!(
        view.to_screen(BoardPoint { x: 210.0, y: 280.0 }),
        ScreenPoint { x: 680.0, y: 320.0 }
    );
}

#[test]
fn viewport_zoom_anchor_property_over_generated_asymmetric_inputs() {
    // Deterministic generative property test, not fixtures restricted to the center.
    let mut seed = 0x93c4_a762_015e_88b9_u64;
    let mut sample = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / ((1_u64 << 53) as f64)
    };
    for _ in 0..10_000 {
        let center = BoardPoint {
            x: sample() * 14310.0,
            y: sample() * 4632.0,
        };
        let size = [500.0 + sample() * 2000.0, 300.0 + sample() * 1200.0];
        let start_zoom = 0.05 + sample() * 3.95;
        let end_zoom = 0.05 + sample() * 3.95;
        let cursor = ScreenPoint {
            x: sample() * size[0],
            y: sample() * size[1],
        };
        let mut view = Viewport::new(center, start_zoom, size, 1.0 + sample()).unwrap();
        let anchor = view.to_board(cursor);
        view.zoom_about(cursor, end_zoom).unwrap();
        let projected = view.to_screen(anchor);
        close(projected.x, cursor.x);
        close(projected.y, cursor.y);
        close(view.zoom(), end_zoom);
        view.zoom_about(cursor, start_zoom).unwrap();
        close(view.center().x, center.x);
        close(view.center().y, center.y);
    }
}

#[test]
fn viewport_clamp_handles_board_edges_and_letterboxing_separately() {
    let mut view = Viewport::new(
        BoardPoint {
            x: -100.0,
            y: 9000.0,
        },
        1.0,
        [1280.0, 800.0],
        2.0,
    )
    .unwrap();
    view.clamp_to_board([14310.0, 4632.0]).unwrap();
    assert_eq!(
        view.center(),
        BoardPoint {
            x: 640.0,
            y: 4232.0
        }
    );
    let mut small = Viewport::new(
        BoardPoint {
            x: -100.0,
            y: 9000.0,
        },
        0.05,
        [2458.0, 1358.0],
        1.0,
    )
    .unwrap();
    small.clamp_to_board([14310.0, 4632.0]).unwrap();
    assert_eq!(
        small.center(),
        BoardPoint {
            x: 7155.0,
            y: 2316.0
        }
    );
    let mut mixed = Viewport::new(
        BoardPoint {
            x: 9900.0,
            y: -9000.0,
        },
        1.0,
        [1280.0, 800.0],
        1.0,
    )
    .unwrap();
    mixed.clamp_to_board([1000.0, 4632.0]).unwrap();
    assert_eq!(mixed.center(), BoardPoint { x: 500.0, y: 400.0 });
    let before = mixed;
    assert!(mixed
        .zoom_about(ScreenPoint { x: 0.0, y: 0.0 }, 0.0)
        .is_err());
    assert!(mixed.clamp_to_board([f64::NAN, 500.0]).is_err());
    assert_eq!(mixed, before, "invalid operations are atomic");
    assert!(Viewport::new(BoardPoint { x: 0.0, y: 0.0 }, 1.0, [1280.0, 800.0], 0.0).is_err());
}
