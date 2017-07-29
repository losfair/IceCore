extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .flag("-std=c++11")
        .file("src/glue/general.cpp")
        .compile("libice_glue.a");
}
