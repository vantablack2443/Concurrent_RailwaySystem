use crate::scenarios;

#[test]
fn test_slug_europe() {
    scenarios::EUROPE.test_case().creep()
}

#[test]
fn test_slug_simple() {
    scenarios::SIMPLE.test_case().creep();
}
