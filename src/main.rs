use blank_web::Blank;
use serde::Deserialize;
use serde_json::from_str;
use ureq::post as post_http;

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
            if rq
                .content_type()
                .unwrap_or_default()
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                == "application/json"
            {
                let QambulanceQuickQuery {
                    location:
                        Location {
                            latitude,
                            longitude,
                        },
                } = match rq.body_text() {
                    Ok(body) => match from_str::<QambulanceQuickQuery>(body) {
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

                if !latitude.is_finite()
                    || !longitude.is_finite()
                    || !(-90.0..=90.0).contains(&latitude)
                    || !(-180.0..=180.0).contains(&longitude)
                {
                    out.bad_request().json("{\"error\":true}");
                    return;
                }

                let query = format!(
                    r#"[out:json][timeout:25];
                    (
                        node["amenity"="hospital"](around:5000,{},{});
                        way["amenity"="hospital"](around:5000,{},{});
                        relation["amenity"="hospital"](around:5000,{},{});
                    );
                    out center;"#,
                    latitude, longitude, latitude, longitude, latitude, longitude
                );

                let response = post_http("https://overpass-api.de/api/interpreter")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .send_form([("data", query.as_str())]);

                match response {
                    Ok(response) => match response.into_body().read_to_string() {
                        Ok(body) => {
                            out.json(&body);
                        }
                        Err(_) => {
                            out.json(r#"{"error":"Failed to read Overpass response"}"#);
                        }
                    },
                    Err(_) => {
                        out.json(r#"{"error":"Overpass API request failed"}"#);
                    }
                }
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
