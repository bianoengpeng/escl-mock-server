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
use actix_web::middleware::Logger;
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
            
            // 获取本机实际 IP 地址
            let local_ip = match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => {
                    match socket.connect("8.8.8.8:80") {
                        Ok(_) => {
                            match socket.local_addr() {
                                Ok(addr) => addr.ip().to_string(),
                                Err(_) => args.binding_address.clone(),
                            }
                        },
                        Err(_) => args.binding_address.clone(),
                    }
                },
                Err(_) => args.binding_address.clone(),
            };
            
            println!("Using local IP: {}", local_ip);
            
            // 使用更简单的主机名格式
            let hostname = format!("escl-mock-scanner-{}.local.", 
                                  local_ip.replace(".", "-"));
            
            println!("Using hostname: {}", hostname);
            
            // 创建符合 Windows 11 要求的 TXT 记录
            let adminurl = format!("http://{}:{}{}", local_ip, args.port, args.scope);
            let representation = format!("http://{}:{}/icon.png", local_ip, args.port);
            
            let txt_records = vec![
                ("txtvers", "1"),
                ("ty", "eSCL Mock Scanner"),
                ("rs", "eSCL"), 
                ("vers", "2.6"),
                ("pdl", "application/pdf,image/jpeg,application/octet-stream"),
                ("cs", "color,grayscale,binary"),
                ("is", "platen,adf"),
                ("duplex", "F"),
                ("note", "Mock eSCL Scanner"),
                ("adminurl", adminurl.as_str()),
                ("representation", representation.as_str()),
                ("uuid", "550e8400-e29b-41d4-a716-446655440000"),
                ("mfg", "MockCompany"),
                ("mdl", "eSCL-Mock-Scanner"),
                ("usb_MFG", "MockCompany"),
                ("usb_MDL", "eSCL Mock Scanner"),
                // Windows 11 特定字段
                ("product", "(eSCL Mock Scanner)"),
                ("priority", "50"),
                ("qtotal", "1"),
                ("scan", "T"),
                ("Scan", "T"),
                ("Color", "T"),
                ("Duplex", "F"),
                ("Transparent", "T"),
                ("kind", "document,photo"),
                ("PaperMax", "legal-A4"),
                ("URF", "none"),
                ("rp", "eSCL"),
                ("air", "username,password"),
                ("UUID", "550e8400-e29b-41d4-a716-446655440000"),
                // 添加 Windows 设备类别信息
                ("printer-type", "0x809046"),  // 网络扫描仪类型
                ("printer-state", "3"),        // 空闲状态
                ("printer-state-reasons", "none"),
                ("device-class", "hardcopy"),
                ("device-kind", "scanner"),
                ("device-make-and-model", "MockCompany eSCL Mock Scanner"),
                ("device-uuid", "550e8400-e29b-41d4-a716-446655440000"),
            ];
            
            // 注册主要的 _uscan._tcp 服务
            match ServiceInfo::new(
                "_uscan._tcp.local.",
                "eSCL Mock Scanner",
                &hostname,
                &local_ip,
                args.port,
                &txt_records[..],
            ) {
                Ok(service_info) => {
                    println!("Service info created successfully");
                    match mdns.register(service_info) {
                        Ok(_) => {
                            println!("mDNS _uscan._tcp service registered successfully");
                        },
                        Err(e) => println!("Failed to register _uscan._tcp service: {}", e),
                    }
                },
                Err(e) => println!("Failed to create _uscan._tcp service info: {}", e),
            }
            
            // 也注册 _uscans._tcp 服务（安全版本）- 使用不同的服务名称
            match ServiceInfo::new(
                "_uscans._tcp.local.",
                "eSCL Mock Scanner Secure",
                &hostname,
                &local_ip,
                args.port,
                &txt_records[..],
            ) {
                Ok(service_info) => {
                    match mdns.register(service_info) {
                        Ok(_) => {
                            println!("mDNS _uscans._tcp service registered successfully");
                        },
                        Err(e) => println!("Failed to register _uscans._tcp service: {}", e),
                    }
                },
                Err(e) => println!("Failed to create _uscans._tcp service info: {}", e),
            }
            
            // 注册 HTTP 服务以提供设备描述
            match ServiceInfo::new(
                "_http._tcp.local.",
                "eSCL Mock Scanner Web",
                &hostname,
                &local_ip,
                args.port,
                &[
                    ("path", "/device.xml"),
                    ("ty", "eSCL Mock Scanner"),
                    ("note", "Device Description"),
                ][..],
            ) {
                Ok(service_info) => {
                    match mdns.register(service_info) {
                        Ok(_) => {
                            println!("mDNS HTTP service registered successfully");
                        },
                        Err(e) => println!("Failed to register HTTP service: {}", e),
                    }
                },
                Err(e) => println!("Failed to create HTTP service info: {}", e),
            }
            
            println!("Service available at: http://{}:{}{}", local_ip, args.port, args.scope);
            
            // 保持 mDNS 服务活跃
            std::mem::forget(mdns);
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
            .wrap(Logger::default())  // 添加详细的请求日志
            .app_data(app_data.clone())
            .service(escl_server::scanner_icon)  // 图标端点在根路径
            .service(escl_server::root_info)     // 根路径设备信息
            .service(escl_server::wsd_description) // WSD 设备描述
            .service(escl_server::device_metadata) // Windows 设备元数据
            .service(escl_server::ssdp_description) // SSDP 发现支持
            .service(
                web::scope(&scope)
                    .service(escl_server::scanner_capabilities)
                    .service(escl_server::scanner_status)
                    .service(escl_server::device_info)  // 添加设备信息端点
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
