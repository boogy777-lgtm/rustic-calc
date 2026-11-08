//! Logical geometry is converted to physical pixels exactly once.
pub const BASE_DPI: u32 = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DpiScale(u32);

impl DpiScale {
    /// A failed Windows DPI query returns zero; only that value uses a fallback.
    pub fn new(dpi: u32) -> Self {
        Self(if dpi == 0 { BASE_DPI } else { dpi })
    }

    pub fn dpi(self) -> u32 {
        self.0
    }

    pub fn factor(self) -> f32 {
        self.0 as f32 / BASE_DPI as f32
    }

    pub fn px(self, logical_dp: f32) -> i32 {
        dp_to_px(logical_dp, self.factor())
    }

    /// Conservative conversion of a measured physical minimum, not geometry.
    pub fn minimum_dp(self, measured_px: i32) -> i32 {
        (measured_px as f32 / self.factor()).ceil() as i32
    }
}

pub fn dp_to_px(logical_dp: f32, factor: f32) -> i32 {
    debug_assert!(factor.is_finite() && factor > 0.0);
    (logical_dp * factor).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_is_a_multiplier_including_below_100_percent() {
        for dpi in [72, 96, 120, 144, 168, 192, 240, 288, 384] {
            let scale = DpiScale::new(dpi);
            assert_eq!(scale.px(BASE_DPI as f32), dpi as i32);
            assert_eq!(scale.factor(), dpi as f32 / BASE_DPI as f32);
            for measured in [1, 17, 51, 287, 483] {
                assert!(scale.px(scale.minimum_dp(measured) as f32) >= measured);
            }
        }
        assert_eq!(DpiScale::new(0).dpi(), BASE_DPI);
    }

    #[test]
    fn transitions_always_start_from_logical_geometry() {
        let source_dp = [3.0, 8.0, 51.0, 287.0, 286.0];
        let initial: Vec<_> = source_dp.iter().map(|&v| DpiScale::new(96).px(v)).collect();
        for dpi in [96, 144, 192, 96, 120, 96] {
            let result: Vec<_> = source_dp
                .iter()
                .map(|&v| DpiScale::new(dpi).px(v))
                .collect();
            if dpi == 96 {
                assert_eq!(result, initial);
            }
        }
    }
}
