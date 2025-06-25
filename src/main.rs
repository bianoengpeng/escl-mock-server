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

use crate::model::{ScanJob, ScanSource};
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;
use mdns_sd::{ServiceDaemon, ServiceInfo};

struct AppState {
    scanner_caps: String,
    image_path: Option<String>,
    scan_jobs: Mutex<HashMap<Uuid, ScanJob>>,
    scan_sources: Mutex<HashMap<Uuid, ScanSource>>,  // 存储每个扫描任务的扫描源
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting escl-mock-server...");
    let args = cli::parse_cli();
    println!("CLI parsing completed");

    println!("Configuration: {args}");

    let scanner_caps = match args.scanner_caps_file {
        Some(file) => std::fs::read_to_string(file).expect("Couldn't read specified file"),
        None => include_str!("../res/default_scanner_caps.xml").to_owned(),
    };

    let app_data = web::Data::new(AppState {
        scanner_caps,
        image_path: args.served_image,
        scan_jobs: Mutex::new(HashMap::new()),
        scan_sources: Mutex::new(HashMap::new()),
    });

    // 克隆需要在多个地方使用的值
    let binding_address = args.binding_address.clone();
    let scope = args.scope.clone();
    let port = args.port;

    // 尝试设置 mDNS 服务（如果失败则继续运行）
    println!("Attempting to create mDNS daemon...");
    match ServiceDaemon::new() {
        Ok(mdns) => {
            println!("mDNS daemon created successfully");
            
            // 获取主机名（不使用IP地址）
            let hostname = match std::env::var("COMPUTERNAME") {
                Ok(name) => format!("{}.local.", name.to_lowercase()),
                Err(_) => "escl-mock-scanner.local.".to_string(),
            };
            
            println!("Using hostname: {}", hostname);
            
            // 创建完整的 TXT 记录 - 使用 String 确保生命周期
            let adminurl = format!("http://{}:{}{}", args.binding_address, args.port, args.scope);
            let txt_records = vec![
                ("txtvers", "1"),
                ("ty", "eSCL Mock Scanner"),
                ("rs", "eSCL"), 
                ("vers", "2.0"),
                ("pdl", "application/pdf,image/jpeg"),
                ("cs", "color,grayscale,binary"),
                ("is", "platen"),
                ("duplex", "F"),
                ("note", "Mock eSCL Scanner"),
                ("adminurl", adminurl.as_str()),
                ("uuid", "550e8400-e29b-41d4-a716-446655440000"),
                ("mfg", "Mock"),
                ("mdl", "eSCL Scanner"),
            ];
            
            match ServiceInfo::new(
                "_uscan._tcp.local.",
                "eSCL Mock Scanner",
                &hostname,
                &args.binding_address,
                args.port,
                &txt_records[..],
            ) {
                Ok(service_info) => {
                    println!("Service info created successfully");
                    match mdns.register(service_info) {
                        Ok(_) => {
                            println!("mDNS service registered successfully");
                            println!("Service available at: http://{}:{}{}", args.binding_address, args.port, args.scope);
                        },
                        Err(e) => println!("Failed to register mDNS service: {}", e),
                    }
                },
                Err(e) => println!("Failed to create service info: {}", e),
            }
        },
        Err(e) => {
            println!("Failed to create mDNS daemon: {}", e);
            println!("Continuing without mDNS service discovery...");
        }
    }

    // 然后启动 HTTP 服务器
    println!("Starting HTTP server on {}:{}", args.binding_address, args.port);
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(
                web::scope(&scope)
                    .service(escl_server::scanner_capabilities)
                    .service(escl_server::scanner_status)
                    .service(escl_server::scan_job)
                    .service(escl_server::next_doc),
            )
            .default_service(web::route().to(escl_server::not_found))
    })
    .bind((binding_address, port))
    .expect("Couldn't create HTTP server")
    .run()
    .await
}
