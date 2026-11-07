use crate::scenarios;

mod test_rocket;
mod test_slug;

/// Makes sure that the macros for defining test scenarios work as expected.
#[test]
pub fn test_macros() {
    scenarios::SIMPLE.load_scenario();
    scenarios::SIMPLE.load_output();
}
