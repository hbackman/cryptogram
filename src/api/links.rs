use serde::{Serialize, Deserialize};
use warp::Filter;
use scraper::{Html, Selector};

#[derive(Clone, Serialize)]
struct LinkPreview {
  image:       Option<String>,
  title:       Option<String>,
  description: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
struct LinkPreviewRequest {
  link: String,
}

pub fn link_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path!("link_preview")
    .and(warp::query::raw())
    .and(warp::get())
    .and_then(handle_preview)
}

async fn handle_preview(query: String) -> Result<impl warp::Reply, warp::Rejection> {
  let query = serde_qs::from_str::<LinkPreviewRequest>(&query)
    .unwrap();

  match fetch_link_preview(&query.link).await {
    Ok(preview) => {
      Ok(warp::reply::json(&preview))
    },
    Err(_) => {
      Err(warp::reject::not_found())
    }
  }
}

async fn fetch_link_preview(url: &str) -> Result<LinkPreview, String> {
  let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
  let contents = response.text().await.map_err(|e| e.to_string())?;

  let document = Html::parse_document(&contents);

  let title_selector = Selector::parse(r#"meta[property="og:title"]"#).unwrap();
  let desc_selector  = Selector::parse(r#"meta[property="og:description"]"#).unwrap();
  let image_selector = Selector::parse(r#"meta[property="og:image"]"#).unwrap();

  let title = document
    .select(&title_selector)
    .next()
    .and_then(|el| el.value().attr("content"))
    .map(String::from);

  let description = document
    .select(&desc_selector)
    .next()
    .and_then(|el| el.value().attr("content"))
    .map(String::from);

  let image = document
    .select(&image_selector)
    .next()
    .and_then(|el| el.value().attr("content"))
    .map(String::from);

  Ok(LinkPreview{image, title, description})
}
