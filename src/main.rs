use blank_web::Blank;

fn main() {
    let mut server = Blank::new();

    server.route_func("/", |_rq, out| {
        out.json(
            "{\
        \"message\": \"Hello, Bro!\"\
       }",
        );
    });

    server.port(5748);
    println!(
        "Listening on http://{}:{}",
        server.get_host(),
        server.get_port()
    );

    server.serve_await().unwrap();
}
