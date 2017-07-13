extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .file("src/glue/glue.cpp")
        .compile("libice_glue.a");
    
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .file("src/internal/prefix_tree.cpp")
        .compile("libice_internal_prefix_tree.a");
}
