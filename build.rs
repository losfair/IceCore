extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .flag("-std=c++11")
        .file("src/glue/general.cpp")
        .file("src/glue/response.cpp")
        .compile("libice_glue.a");
}
