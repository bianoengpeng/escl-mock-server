/*
 *     Copyright (C) 2024-2025 Christian Nagel and contributors
 *
 *     This file is part of escl-mock-server.
 *
 *     escl-mock-server is free software: you can redistribute it and/or modify it under the terms of
 *     the GNU General Public License as published by the Free Software Foundation, either
 *     version 3 of the License, or (at your option) any later version.
 *
 *     escl-mock-server is distributed in the hope that it will be useful, but WITHOUT ANY
 *     WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 *     FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 *     You should have received a copy of the GNU General Public License along with eSCLKt.
 *     If not, see <https://www.gnu.org/licenses/>.
 *
 *     SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::model::ScanJob;
use crate::AppState;
use actix_web::http::{header, StatusCode};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use std::str::FromStr;
use tokio_util::io::ReaderStream;
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
        .insert_header((header::LOCATION, format!("{full_url}/{generated_uuid}")))
        .finish()
}

#[get("/ScanJobs/{uuid}/NextDocument")]
async fn next_doc(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    let full_url = req.full_url();

    println!("Document is retrieved");
    let mut data_guard = data.scan_jobs.lock().await;
    let uuid = &Uuid::from_str(&path.into_inner()).unwrap();

    match data_guard.get_mut(uuid) {
        None => {
            data_guard.insert(*uuid, ScanJob { retrieved_pages: 1 });
        }
        Some(job) => {
            job.retrieved_pages += 1;
        }
    }

    println!("Document job data: {}", data_guard.get(uuid).unwrap());

    if data_guard.get(uuid).unwrap().retrieved_pages > 20 {
        return HttpResponse::NotFound().finish();
    }

    if data.image_path.is_some() {
        let file = tokio::fs::File::open(data.image_path.as_ref().unwrap()).await;
        let stream = ReaderStream::new(file.unwrap());
        HttpResponse::Ok()
            .content_type("image/jpeg")
            .insert_header((header::CONTENT_LOCATION, format!("{full_url}")))
            .streaming(stream)
    } else {
        HttpResponse::Ok()
            .content_type("image/jpeg")
            .insert_header((header::CONTENT_LOCATION, format!("{full_url}")))
            .body(&include_bytes!("../res/example_image.jpg")[..])
    }
}

pub(crate) async fn not_found(req: HttpRequest) -> impl Responder {
    println!(
        "The following path was accessed but is not implemented: {}",
        req.path()
    );

    HttpResponse::build(StatusCode::NOT_FOUND).body("Not found 404")
}
