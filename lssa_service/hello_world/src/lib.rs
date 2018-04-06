#[macro_use]
extern crate lssa_service;

app_init!({
    println!("Hello world");
    lssa_service::set_timeout(1000, || {
        eprintln!("Hello world 2");
    });
    lssa_service::schedule(|| {
        eprintln!("Hello world 3");
    });
    0
});
