extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .flag("-std=c++11")
        .file("src/glue/general.cpp")
        .file("src/glue/request.cpp")
        .file("src/glue/response.cpp")
        .compile("libice_glue.a");
    
    gcc::Config::new()
        .cpp(true)
        .opt_level(3)
        .flag("-std=c++11")
        .file("src/internal/prefix_tree.cpp")
        .compile("libice_internal_prefix_tree.a");
}
