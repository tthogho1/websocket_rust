use askama::Template;
use axum::response::Html;
use axum::response::IntoResponse;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
    ws_server: &'a str
}

pub fn render_template(name: String,ws_server: String) ->  impl IntoResponse {
    // テンプレートのレンダリングロジック
    let template = HelloTemplate { name: &name, ws_server:&ws_server };
    let rendered = template.render().unwrap();

    return Html(rendered);
}