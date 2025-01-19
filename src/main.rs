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

mod cli;
mod escl_server;
mod model;

use crate::model::ScanJob;
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

struct AppState {
    scanner_caps: String,
    image_path: Option<String>,
    scan_jobs: Mutex<HashMap<Uuid, ScanJob>>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = cli::parse_cli();

    println!("Configuration: {args}");

    let scanner_caps = match args.scanner_caps_file {
        Some(file) => std::fs::read_to_string(file).expect("Couldn't read specified file"),
        None => include_str!("../res/default_scanner_caps.xml").to_owned(),
    };

    let app_data = web::Data::new(AppState {
        scanner_caps,
        image_path: args.served_image,
        scan_jobs: Mutex::new(HashMap::new())
    });

    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(web::scope("/eSCL")
                .service(escl_server::scanner_capabilities)
                .service(escl_server::scan_job)
                .service(escl_server::next_doc)
            )
            .default_service(web::route().to(escl_server::not_found))
    })
    .bind((args.binding_address, args.port))
    .expect("Couldn't create HTTP server")
    .run();

    http_server.await
}
