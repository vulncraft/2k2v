pub fn add_two(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_test() {
        let x = 1;
        let y = 2;
        assert_eq!(x + y, add_two(x, y));
    }
}
