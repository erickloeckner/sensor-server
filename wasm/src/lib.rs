// build command:
// wasm-pack build --release --target web

use futures_util::stream::StreamExt;
use gloo_timers::future::IntervalStream;
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{console, Document, Element, Request, RequestInit, RequestMode, Response, Window};

#[derive(Serialize, Deserialize)]
struct GraphData {
    data: f32,
    time: u64,
}

#[derive(Serialize, Deserialize)]
struct DataVec(Vec<GraphData>);

fn window() -> Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> Document {
    window().document().expect("should have a document on window")
}

async fn set_graph(request: &Request, line: &Element, text: &Element) {
    //let resp_value = JsFuture::from(window().fetch_with_request(&request)).await;
    let mut line_string = String::new();
    line_string.push('M');
        match get_data(&request).await {
            Ok(i) => {
                let y_min: f32 = 75.0;
                let y_max: f32 = 300.0;
                let y_range: f32 = y_max - y_min;

                for (index, value) in i.0.iter().enumerate() {
                    let x_pos = (index as f32) / ((i.0.len() - 1) as f32);
                    let x_pos_scale = x_pos * 100.0;
                    
                    //let y_pos = 100.0 - value.data;
                    let y_pos_scale = ((value.data - y_min) / y_range) * 100.0;
                    let y_pos = 100.0 - y_pos_scale;
                    
                    if index == 0 {
                        line_string.push_str(&format!(" -10.0,{}", y_pos));
                    }
                    line_string.push_str(&format!(" {},{}", x_pos_scale, y_pos));
                    if index == (i.0.len() - 1) {
                        line_string.push_str(&format!(" 110.0,{}", y_pos));
                        text.set_inner_html(&format!("{}", value.data));
                    }
                }
            }
            Err(_e) => console::log_1(&"error parsing graph JSON".into())
        }
    line.set_attribute("d", &line_string).unwrap();
}

async fn get_data(request: &Request) -> Result<DataVec, serde_wasm_bindgen::Error> {
    let resp_value = JsFuture::from(window().fetch_with_request(&request)).await;
    match resp_value {
        Ok(v) => {
            let resp: Response = v.dyn_into().unwrap();
            let resp_json = JsFuture::from(resp.json().unwrap()).await.unwrap();
            serde_wasm_bindgen::from_value(resp_json)
        }
        Err(e) => Err(e.into()),
    }
}

async fn reset_data() {
    let url = format!("{}/reset", &window_url());
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&url, &opts).unwrap();
    let resp_value = JsFuture::from(window().fetch_with_request(&request)).await;
    match resp_value {
        Ok(_v) => {
            console::log_1(&"data reset".into());
        }
        Err(e) => {
            console::log_1((&e).into());
        }
    }
}

fn window_url() -> String {
    let win_proto = window().location().protocol().unwrap();
    let win_host = window().location().host().unwrap();
    format!("{}//{}", win_proto, win_host)
}

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    spawn_local(async {
        let data_url = format!("{}/get_data", &window_url());

        let graph_line = document().get_element_by_id("graph_line").unwrap();
        let graph_text = document().get_element_by_id("graph_text").unwrap();

        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(&data_url, &opts).unwrap();
        request.headers().set("Accept", "application/json").unwrap();

        set_graph(&request, &graph_line, &graph_text).await;

        IntervalStream::new(15_000).for_each( |_| {
            set_graph(&request, &graph_line, &graph_text)
        }).await;
    });
    
    let reset_btn = document()
        .get_element_by_id("reset")
        .expect("unable to get send element");
    let reset_btn_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        spawn_local(async {
            reset_data().await;
        });
    }) as Box<dyn FnMut(_)>);
    reset_btn.add_event_listener_with_callback("mousedown", reset_btn_callback.as_ref().unchecked_ref())?;
    reset_btn_callback.forget();

    console::log_1(&"Wasm console log()".into());
    Ok(())
}
