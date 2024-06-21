// build command:
// cargo build --release

use std::collections::VecDeque;
use std::env;
use std::fs;
use std::process;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde_derive::{Deserialize, Serialize};
use warp::Filter;

#[derive(Deserialize, Serialize, Debug)]
struct SensorData {
    data: f32,
}

#[derive(Deserialize, Serialize, Debug)]
struct GraphData {
    data: f32,
    time: u64,
}

#[derive(Deserialize)]
struct Config {
    debug: bool,
    port: u16,
    value_count: usize,
    value_default: f32,
}

#[tokio::main]
async fn main() {
    let config_path = env::args().nth(1).unwrap_or_else(|| {
        println!("no config file specified");
        process::exit(1);
    });
    let config_raw = fs::read_to_string(&config_path).unwrap_or_else(|err| {
        println!("error reading config: {}", err);
        process::exit(1);
    });
    let config: Config = toml::from_str(&config_raw).unwrap_or_else(|err| {
        println!("error parsing config: {}", err);
        process::exit(1);
    });

    //let debug_clone = config.debug.clone();
    let port_clone = config.port.clone();

    let sensor_data_vec: Arc<Mutex<VecDeque<GraphData>>> = Arc::new(Mutex::new(VecDeque::new()));
    for _ in 0..config.value_count {
        sensor_data_vec.lock().unwrap().push_back(GraphData {data: config.value_default, time: 0 });
    }
    let sensor_data_vec = warp::any().map(move || sensor_data_vec.clone());

    let config_arc = Arc::new(Mutex::new(config));
    let config_filter = warp::any().map(move || config_arc.clone());

    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("./static/index.html"));

    let style = warp::get()
        .and(warp::path("style.css"))
        .and(warp::fs::file("./static/style.css"));

    let pkg = warp::path("pkg")
        .and(warp::fs::dir("./wasm/pkg/"));

    let submit = warp::post()
        .and(warp::path("submit"))
        .and(warp::body::content_length_limit(500))
        .and(warp::body::json())
        .and(sensor_data_vec.clone())
        .and(config_filter.clone())
        .map(move |data: SensorData, data_vec: Arc<Mutex<VecDeque<GraphData>>>, config: Arc<Mutex<Config>>| {
            let mut data_vec_inner = match data_vec.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let config_inner = match config.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let ts: u64 = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n.as_secs(),
                Err(_) => 0,
            };
            let graph_data = GraphData { data: data.data, time: ts };
            data_vec_inner.pop_front();
            data_vec_inner.push_back(graph_data);
            drop(data_vec_inner);
            if config_inner.debug { println!("{:?} | {:?}", data, data_vec) }
            drop(config_inner);
            
            format!("OK")
        });

    //let reset = warp::post()
        //.and(warp::path("reset"))
        //.and(warp::body::content_length_limit(500))
        //.and(warp::body::json())
    let reset = warp::path("reset")
        .and(sensor_data_vec.clone())
        .and(config_filter.clone())
        //.map(move |data: SensorData, data_vec: Arc<Mutex<VecDeque<GraphData>>>, config: | {
        .map(move |data_vec: Arc<Mutex<VecDeque<GraphData>>>, config: Arc<Mutex<Config>>| {
            let mut data_vec_inner = match data_vec.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let config_inner = match config.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            for _ in 0..config_inner.value_count {
                data_vec_inner.pop_front();
                data_vec_inner.push_back(GraphData {data: config_inner.value_default, time: 0 });
            }
            drop(data_vec_inner);
            if config_inner.debug { println!("data reset") }
            drop(config_inner);
            
            format!("OK")
        });

    let get_data = warp::get()
        .and(warp::path("get_data"))
        .and(sensor_data_vec.clone())
        .map(|data_vec: Arc<Mutex<VecDeque<GraphData>>>| {
            let mut data_out = Vec::new();
            let data_vec_inner = data_vec.lock().unwrap();
            for i in data_vec_inner.iter() {
                data_out.push(i);
            }
            warp::reply::json(&data_out)
        });

    let routes = index
        .or(style)
        .or(pkg)
        .or(submit)
        .or(reset)
        .or(get_data);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port_clone))
        .await;
}
