use crate::AppState;
use actix_web::http::{header, StatusCode};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use uuid::Uuid;

#[get("/ScannerCapabilities")]
async fn scanner_capabilities(data: web::Data<AppState>) -> impl Responder {
    println!("ScannerCaps downloaded");

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(data.scanner_caps.to_owned())
}

#[post("/ScanJobs")]
async fn scan_job(req: HttpRequest) -> impl Responder {

    let full_url = req.full_url();
    let generated_uuid = Uuid::new_v4();

    HttpResponse::build(StatusCode::CREATED)
        .insert_header(
            (header::LOCATION, format!("{full_url}/{generated_uuid}"))
        )
        .finish()
}

#[get("/ScanJobs/{uuid}/NextDocument")]
async fn next_doc() -> impl Responder {
    println!("Document is retrieved");
    ""
}

pub(crate) async fn not_found(req: HttpRequest) -> impl Responder {
    println!("The following path was accessed but is not implemented: {}", req.path());

    HttpResponse::build(StatusCode::NOT_FOUND)
        .body("Not found 404")
}