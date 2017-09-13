pub fn normalize_path<'a>(p: &'a str, default_p: &'a str) -> (Vec<&'a str>, Vec<&'a str>) {
    let mut param_names = Vec::new();
    let path = p.split("/").filter(|v| v.len() > 0).map(|v| {
        if v.starts_with(":") {
            param_names.push(&v[1..]);
            default_p
        } else {
            v
        }
    }).collect();
    (path, param_names)
}
