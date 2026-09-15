//! Brand-mark easter-egg state.

/// Counts logo clicks; every seventh click toggles the gold Suisei mark tint.
/// `clicks` resets each time the toggle fires.
#[derive(Debug, Default, Clone, Copy)]
pub struct EasterEgg {
    clicks: u32,
    shown: bool,
}

/// How many clicks toggle the mark.
const CROWN_CLICKS: u32 = 7;

impl EasterEgg {
    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// Register one logo click. Returns `true` only on the click that turns the
    /// mark on; turning it off returns `false` like every non-toggling click.
    pub fn register_click(&mut self) -> bool {
        self.clicks += 1;
        if self.clicks < CROWN_CLICKS {
            return false;
        }
        self.clicks = 0;
        self.shown = !self.shown;
        self.shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventh_click_toggles_on_then_off_without_revert_signal() {
        let mut egg = EasterEgg::default();
        for _ in 0..6 {
            assert!(!egg.register_click());
            assert!(!egg.is_shown());
        }

        assert!(egg.register_click());
        assert!(egg.is_shown());

        for _ in 0..6 {
            assert!(!egg.register_click());
            assert!(egg.is_shown());
        }

        assert!(!egg.register_click());
        assert!(!egg.is_shown());
    }
}
