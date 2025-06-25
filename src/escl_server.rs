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

use crate::model::{ScanJob, ScanSource};
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

#[get("/ScannerStatus")]
async fn scanner_status() -> impl Responder {
    println!("ScannerStatus requested");

    let status_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScannerStatus xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" 
                    xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03" 
                    xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm" 
                    xsi:schemaLocation="http://schemas.hp.com/imaging/escl/2011/05/03 eSCL.xsd">
    <pwg:Version>2.0</pwg:Version>
    <pwg:State>Idle</pwg:State>
    <scan:ScannerState>Idle</scan:ScannerState>
    <scan:ScannerStateReasons>
        <scan:ScannerStateReason>None</scan:ScannerStateReason>
    </scan:ScannerStateReasons>
</scan:ScannerStatus>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(status_xml)
}

#[post("/ScanJobs")]
async fn scan_job(req: HttpRequest, body: web::Bytes, data: web::Data<AppState>) -> impl Responder {
    let full_url = req.full_url();
    let generated_uuid = Uuid::new_v4();
    
    // 尝试解析扫描请求以确定扫描源
    let scan_source = if let Ok(body_str) = std::str::from_utf8(&body) {
        println!("Scan request body: {}", body_str);
        if body_str.contains("<scan:InputSource>Adf</scan:InputSource>") 
           || body_str.contains("Feeder") 
           || body_str.contains("ADF") {
            println!("Detected ADF scan request");
            ScanSource::Adf
        } else {
            println!("Detected Platen scan request");
            ScanSource::Platen
        }
    } else {
        println!("Could not parse scan request, defaulting to Platen");
        ScanSource::Platen
    };
    
    // 保存扫描源信息
    {
        let mut sources_guard = data.scan_sources.lock().await;
        sources_guard.insert(generated_uuid, scan_source);
    }

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

    // 获取扫描源信息
    let scan_source = {
        let sources_guard = data.scan_sources.lock().await;
        sources_guard.get(uuid).cloned().unwrap_or(ScanSource::Platen)
    };

    match data_guard.get_mut(uuid) {
        None => {
            // 新的扫描任务
            let max_pages = match scan_source {
                ScanSource::Platen => 1,
                ScanSource::Adf => 5,  // ADF模拟最多5页
            };
            data_guard.insert(*uuid, ScanJob { 
                retrieved_pages: 1,
                scan_source: scan_source.clone(),
                max_pages,
            });
        }
        Some(job) => {
            job.retrieved_pages += 1;
        }
    }

    let current_job = data_guard.get(uuid).unwrap();
    println!("Document job data: {}", current_job);

    // 根据扫描源确定是否还有更多页面
    let has_more_pages = current_job.retrieved_pages <= current_job.max_pages;

    if !has_more_pages {
        println!("No more pages available for {:?} source", current_job.scan_source);
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
