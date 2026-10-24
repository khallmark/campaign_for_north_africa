//! DOM-free transforms in CSS pixels. DPR affects raster resolution, not hit coordinates.

use crate::geometry::BoardPoint;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    center: BoardPoint,
    zoom: f64,
    css_size: [f64; 2],
    device_pixel_ratio: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidViewport;

impl fmt::Display for InvalidViewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("viewport coordinates must be finite; sizes, zoom and DPR must be positive")
    }
}

impl Error for InvalidViewport {}

impl Viewport {
    /// `zoom` is CSS pixels per board pixel; `css_size` is [width, height].
    pub fn new(
        center: BoardPoint,
        zoom: f64,
        css_size: [f64; 2],
        device_pixel_ratio: f64,
    ) -> Result<Self, InvalidViewport> {
        if !center.x.is_finite()
            || !center.y.is_finite()
            || [zoom, css_size[0], css_size[1], device_pixel_ratio]
                .into_iter()
                .any(|v| !v.is_finite() || v <= 0.0)
        {
            return Err(InvalidViewport);
        }
        Ok(Self {
            center,
            zoom,
            css_size,
            device_pixel_ratio,
        })
    }

    pub const fn center(self) -> BoardPoint {
        self.center
    }
    pub const fn zoom(self) -> f64 {
        self.zoom
    }
    pub const fn css_size(self) -> [f64; 2] {
        self.css_size
    }
    pub const fn device_pixel_ratio(self) -> f64 {
        self.device_pixel_ratio
    }

    pub fn to_screen(self, point: BoardPoint) -> ScreenPoint {
        ScreenPoint {
            x: (point.x - self.center.x) * self.zoom + self.css_size[0] / 2.0,
            y: (point.y - self.center.y) * self.zoom + self.css_size[1] / 2.0,
        }
    }

    pub fn to_board(self, point: ScreenPoint) -> BoardPoint {
        BoardPoint {
            x: (point.x - self.css_size[0] / 2.0) / self.zoom + self.center.x,
            y: (point.y - self.css_size[1] / 2.0) / self.zoom + self.center.y,
        }
    }

    /// Preserve the board point under the cursor. Clamping is a separate operation:
    /// when the board edge limits panning, clamping cannot also preserve this anchor.
    pub fn zoom_about(&mut self, point: ScreenPoint, zoom: f64) -> Result<(), InvalidViewport> {
        if !point.x.is_finite() || !point.y.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
            return Err(InvalidViewport);
        }
        let anchor = self.to_board(point);
        let center = BoardPoint {
            x: anchor.x - (point.x - self.css_size[0] / 2.0) / zoom,
            y: anchor.y - (point.y - self.css_size[1] / 2.0) / zoom,
        };
        *self = Self::new(center, zoom, self.css_size, self.device_pixel_ratio)?;
        Ok(())
    }

    /// Keep the visible rectangle within [0,width] × [0,height]. If an axis of the
    /// board is smaller than the visible span, center that axis (symmetric margins).
    pub fn clamp_to_board(&mut self, board_size: [f64; 2]) -> Result<(), InvalidViewport> {
        if board_size.into_iter().any(|v| !v.is_finite() || v <= 0.0) {
            return Err(InvalidViewport);
        }
        let clamp = |value: f64, axis: usize| {
            let half = self.css_size[axis] / (2.0 * self.zoom);
            if half >= board_size[axis] / 2.0 {
                board_size[axis] / 2.0
            } else {
                value.clamp(half, board_size[axis] - half)
            }
        };
        self.center = BoardPoint {
            x: clamp(self.center.x, 0),
            y: clamp(self.center.y, 1),
        };
        Ok(())
    }
}
