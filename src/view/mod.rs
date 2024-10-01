use askama::Template;
use axum::response::Html;
use axum::response::IntoResponse;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
    port: &'a u16
}

pub fn render_template(name: String,port:u16) ->  impl IntoResponse {
    // テンプレートのレンダリングロジック
    let template = HelloTemplate { name: &name,port:&port };
    let rendered = template.render().unwrap();

    return Html(rendered);
}