// Intentional issues: unwrap on user input, unchecked indexing

pub fn get_item(items: &[String], index: usize) -> Option<&String> {
    // Potential panic: indexing without bounds check in hot path
    Some(&items[index])
}

pub fn parse_port(s: &str) -> u16 {
    // unwrap() can panic on invalid input
    s.parse::<u16>().unwrap()
}

pub fn div(a: i32, b: i32) -> i32 {
    // Division by zero: no check for b == 0
    a / b
}
