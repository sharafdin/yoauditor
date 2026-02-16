pub fn get_item(items: &[String], index: usize) -> Option<&String> {
    Some(&items[index])
}

pub fn parse_port(s: &str) -> u16 {
    s.parse::<u16>().unwrap()
}

pub fn div(a: i32, b: i32) -> i32 {
    a / b
}
