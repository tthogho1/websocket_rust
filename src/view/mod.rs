use askama::Template;
use axum::response::Html;
use axum::response::IntoResponse;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

pub fn render_template(data: String) ->  impl IntoResponse {
    // テンプレートのレンダリングロジック
    let template = HelloTemplate { name: &data };
    let rendered = template.render().unwrap();

    return Html(rendered);
}