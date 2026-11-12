use blank_web::{
    Blank,
    constants::{
        HTTP_CONTENT_TYPE_HEADER, HTTP_METHOD_GET as HTTP_GET_METHOD,
        HTTP_METHOD_POST as HTTP_POST_METHOD,
    },
};
use serde::Deserialize;
use serde_json::from_str;
use std::env::var as env_var;
use std::process::exit as exit_with_exit_code;
use std::sync::OnceLock;
use std::time::Duration;
use ureq::Agent as HttpAgent;

const HOME_ROUTE: &str = "/";
const QUICK_ROUTE: &str = "/quick";
const HELP_ROUTE: &str = "/help";
const CREDITS_ROUTE: &str = "/credits";

const HTTP_JSON_CONTEXT: &str = "application/json";
const HTTP_FORM_ENCODED_CONTENT: &str = "application/x-www-form-urlencoded";

const DEFAULT_PORT: u16 = 7878;
const HOST: &str = "0.0.0.0";
const PORT_ENVIRONMENT_VAR: &str = "PORT";
const OVERPASS_URL: &str = "https://maps.mail.ru/osm/tools/overpass/api/interpreter";
const OVERPASS_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OVERPASS_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const OVERPASS_RESPONSE_LIMIT: u64 = 2 * 1024 * 1024;

const CREDITS: &str = r#"{
    "credits": [
        "This API was created by": "Kyam (Kushaal Srinandan Reddy B <koolcoolkoolcoolkoolkool@gmail.com>)",
        "Overpass": "This API uses overpass"
    ],
    "thanks for": "asking"
}"#;

const HELLO_MESSAGE: &str = r#"{
    "ok": true,
    "message": "Hello, User! Request /help to get info about this API"
}"#;

const HELP_MESSAGE: &str = r#"{
    "ok": true,
    "message": "Qambulance API - Fast Responses for Fast Action",
    "description": "Find nearby hospitals using geographical coordinates.",
    "endpoints": {
        "/": {
            "method": "GET",
            "description": "Check whether the API is running.",
            "response": {
                "ok": true,
                "message": "Hello, User! Request /help to get info about this API"
            }
        },
        "/quick": {
            "method": "POST",
            "description": "Find hospitals within 5 kilometres of the supplied location.",
            "content_type": "application/json",
            "request_body": {
                "location": {
                    "latitude": "number (-90 to 90)",
                    "longitude": "number (-180 to 180)"
                }
            },
            "response": {
                "ok": true,
                "hospitals": [
                    {
                        "name": "Hospital name",
                        "latitude": "number",
                        "longitude": "number",
                        "distance": "distance in metres"
                    }
                ],
                "msg": "Successfully returned hospitals"
            }
        },
        "/help": {
            "method": "GET",
            "description": "Display API documentation."
        },
        "/credits": {
            "method": "GET",
            "description": "Display API credits and acknowledgements."
        }
    },
    "errors": {
        "400": "Invalid request body, content type, JSON, or coordinates.",
        "405": "HTTP method not allowed.",
        "500": "Internal server error or Overpass API failure."
    },
    "data_source": "OpenStreetMap via Overpass API",
    "distance_unit": "metres",
    "default_search_radius": 5000
}"#;

const INVALID_JSON_STRUCTURE: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Invalid structure, please try again with correct structure"
}"#;

const INVALID_CONTENT_TYPE: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Invalid content type. Use application/json"
}"#;

const INVALID_JSON_BODY: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Invalid JSON request body"
}"#;

const INVALID_LOCATION: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Invalid location coordinates"
}"#;

const OVERPASS_REQUEST_FAILED: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Failed to contact Overpass API"
}"#;

const OVERPASS_RESPONSE_READ_FAILED: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Failed to read Overpass response"
}"#;

const OVERPASS_RESPONSE_PARSE_FAILED: &str = r#"{
    "ok": false,
    "hospitals": [],
    "msg": "Failed to parse Overpass response"
}"#;

macro_rules! invalid_method_response {
    ($method:expr, $valid_method:expr, $use_hospitals:expr, $use_endpoints:expr) => {
        format!(
            "{{ \"ok\": false,{}{} \"{}\": \"Invalid method {}, use {}\" }}",
            if $use_hospitals {
                " \"hospitals\": [],"
            } else {
                ""
            },
            if $use_endpoints {
                "  \"endpoints\": [],"
            } else {
                ""
            },
            if $use_hospitals { "msg" } else { "message" },
            $method,
            $valid_method
        )
    };
}

macro_rules! quick_json_response {
    ($hospitals:expr) => {
        serde_json::json!({
            "hospitals": $hospitals,
            "ok": true,
            "msg": "Successfully returned hospitals"
        })
    };
}

macro_rules! overpass_query {
    ($latitude:expr, $longitude:expr) => {
        format!(
            r#"[out:json][timeout:25];
            (
                node["amenity"="hospital"](around:5000,{},{});
                way["amenity"="hospital"](around:5000,{},{});
                relation["amenity"="hospital"](around:5000,{},{});
            );
            out center;"#,
            $latitude, $longitude, $latitude, $longitude, $latitude, $longitude
        )
    };
}

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

fn api_agent() -> &'static HttpAgent {
    static API_AGENT: OnceLock<HttpAgent> = OnceLock::new();

    API_AGENT.get_or_init(|| {
        HttpAgent::config_builder()
            .timeout_connect(Some(OVERPASS_CONNECT_TIMEOUT))
            .timeout_global(Some(OVERPASS_REQUEST_TIMEOUT))
            .build()
            .into()
    })
}

fn server() -> Blank {
    let mut server = Blank::new();

    server.route_func(HOME_ROUTE, |rq, out| {
        if !rq.is_get() {
            out.method_not_allowed().json(invalid_method_response!(
                rq.method(),
                HTTP_GET_METHOD,
                false,
                false
            ));
            return;
        }
        out.json(HELLO_MESSAGE).ok();
    });

    server.route_func(HELP_ROUTE, |rq, out| {
        if !rq.is_get() {
            out.method_not_allowed().json(invalid_method_response!(
                rq.method(),
                HTTP_GET_METHOD,
                false,
                true
            ));
            return;
        }
        out.json(HELP_MESSAGE).ok();
    });

    server.route_func(QUICK_ROUTE, |rq, out| {
        if rq.is_post() {
            if rq
                .content_type()
                .unwrap_or_default()
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                == HTTP_JSON_CONTEXT
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
                            out.bad_request().json(INVALID_JSON_BODY);
                            return;
                        }
                    },
                    Err(_) => {
                        out.bad_request().json(INVALID_JSON_STRUCTURE);
                        return;
                    }
                };

                if !user_latitude.is_finite()
                    || !user_longitude.is_finite()
                    || !(-90.0..=90.0).contains(&user_latitude)
                    || !(-180.0..=180.0).contains(&user_longitude)
                {
                    out.bad_request().json(INVALID_LOCATION);
                    return;
                }

                let query = overpass_query!(user_latitude, user_longitude);

                let response = api_agent
                    .post(OVERPASS_URL)
                    .header(HTTP_CONTENT_TYPE_HEADER, HTTP_FORM_ENCODED_CONTENT)
                    .send_form([("data", query.as_str())]);

                match response {
                    Ok(response) => match response
                        .into_body()
                        .with_config()
                        .limit(OVERPASS_RESPONSE_LIMIT)
                        .read_to_string()
                    {
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

                                let result = quick_json_response!(hospitals);

                                out.json(result.to_string()).ok();
                            }
                            Err(_) => {
                                out.json(OVERPASS_RESPONSE_PARSE_FAILED)
                                    .internal_server_error();
                            }
                        },
                        Err(_) => {
                            out.json(OVERPASS_RESPONSE_READ_FAILED)
                                .internal_server_error();
                        }
                    },
                    Err(_) => {
                        out.json(OVERPASS_REQUEST_FAILED).internal_server_error();
                    }
                }
            } else {
                out.bad_request().json(INVALID_CONTENT_TYPE);
            }
        } else {
            out.method_not_allowed().json(invalid_method_response!(
                rq.method(),
                HTTP_POST_METHOD,
                true,
                false
            ));
        }
    });

    server.route_func(CREDITS_ROUTE, |_rq, out| {
        out.json(CREDITS).ok();
    });

    server.port(
        env_var(PORT_ENVIRONMENT_VAR)
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT),
    );
    server.host(HOST);
    println!(
        "Listening on http://{}:{}",
        server.get_host(),
        server.get_port()
    );

    server
}

fn main() {
    exit_with_exit_code(match server().serve_await() {
        Ok(_) => 0,
        Err(_) => 1,
    });
}
