use std::ascii::AsciiExt;

pub fn transform_name(v: &str) -> String {
    let mut ret = String::new();
    let mut upper_case = true;

    for ch in v.chars() {
        if upper_case {
            ret.push(ch.to_ascii_uppercase());
            upper_case = false;
        } else {
            ret.push(ch);
        }
        if ch == '-' {
            upper_case = true;
        }
    }

    ret
}
