use crate::scenarios;

#[test]
fn test_rocket_europe_trace() {
    scenarios::EUROPE_TRACE.test_case().launch()
}

#[test]
fn test_rocket_europe() {
    scenarios::EUROPE.test_case().launch()
}

#[test]
fn test_rocket_simple() {
    scenarios::SIMPLE.test_case().launch();
}
