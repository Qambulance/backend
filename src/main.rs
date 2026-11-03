use blank_web::Blank;
use serde::Deserialize;
use serde_json::from_str;

#[derive(Default, Deserialize)]
struct Location {
    latitude: f64,
    longitude: f64,
}

#[derive(Default, Deserialize)]
struct QambulanceQuickQuery {
    location: Location,
}

fn main() {
    let mut server = Blank::new();

    server.route_func("/", |_rq, out| {
        out.json(
            "{\
        \"message\": \"Hello, Bro!\"\
       }",
        );
    });

    server.route_func("/quick", |rq, out| {
        if rq.is_post() {
            if rq.content_type().unwrap_or_default() == "application/json" {
                let QambulanceQuickQuery {
                    location:
                        Location {
                            latitude,
                            longitude,
                        },
                } = match rq.body_text() {
                    Ok(body) => match from_str::<QambulanceQuickQuery>(&body) {
                        Ok(q) => q,
                        Err(_) => {
                            out.bad_request();
                            return;
                        }
                    },
                    Err(_) => {
                        out.bad_request();
                        return;
                    }
                };
            } else {
                out.bad_request();
            }
        } else {
            out.method_not_allowed();
        }
    });

    server.port(5748);
    println!(
        "Listening on http://{}:{}",
        server.get_host(),
        server.get_port()
    );

    server.serve_await().unwrap();
}
