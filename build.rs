extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .file("src/glue/glue.cpp")
        .compile("libice_glue.a");
}
