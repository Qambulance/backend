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

#[derive(Deserialize)]
struct HospitalInfo {
    name: Option<String>,
}

#[derive(Deserialize)]
struct OverpassResponse {
    elements: Vec<Hospital>,
}

#[derive(Deserialize)]
struct Center {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct Hospital {
    lat: Option<f64>,
    lon: Option<f64>,
    center: Option<Center>,
    tags: Option<HospitalInfo>,
}

/// Uses Haversine formula
fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> u64 {
    let earth_radius = 6_371_000.0;

    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();

    let destination_lat = lat2 - lat1;
    let destination_lon = (lon2 - lon1).to_radians();

    let a = (destination_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (destination_lon / 2.0).sin().powi(2);

    (earth_radius * 2.0 * a.sqrt().asin()).round() as u64
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
                            latitude: user_latitude,
                            longitude: user_longitude,
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

                if !user_latitude.is_finite()
                    || !user_longitude.is_finite()
                    || !(-90.0..=90.0).contains(&user_latitude)
                    || !(-180.0..=180.0).contains(&user_longitude)
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
                    user_latitude, user_longitude, user_latitude, user_longitude, user_latitude, user_longitude
                );

                let response = post_http("https://overpass-api.de/api/interpreter")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .send_form([("data", query.as_str())]);

                match response {
                    Ok(response) => match response.into_body().read_to_string() {
                        Ok(body) => match from_str::<OverpassResponse>(&body) {
                            Ok(data) => {
                                let hospitals: Vec<_> = data
                                    .elements
                                    .into_iter()
                                    .filter_map(|hospital| {
                                        let latitude = hospital
                                            .lat
                                            .or_else(|| hospital.center.as_ref().map(|c| c.lat))?;

                                        let longitude = hospital
                                            .lon
                                            .or_else(|| hospital.center.as_ref().map(|c| c.lon))?;

                                        let distance = calculate_distance(
                                            user_latitude,
                                            user_longitude,
                                            latitude,
                                            longitude,
                                        );

                                        let name = hospital
                                            .tags
                                            .and_then(|tags| tags.name)
                                            .unwrap_or_else(|| "Unnamed hospital".to_string());

                                        Some(serde_json::json!({
                                            "latitude": latitude,
                                            "longitude": longitude,
                                            "distance": distance,
                                            "name": name
                                        }))
                                    })
                                    .collect();

                                let result = serde_json::json!({
                                    "hospitals": hospitals
                                });

                                out.json(&result.to_string()).ok();
                            }
                            Err(_) => {
                                out.json(r#"{"error": "Failed to parse overpass response"}"#).internal_server_error();
                            }
                        }
                        Err(_) => {
                            out.json(r#"{"error":"Failed to read Overpass response"}"#).internal_server_error();
                        }
                    },
                    Err(_) => {
                        out.json(r#"{"error":"Overpass API request failed"}"#).internal_server_error();
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
