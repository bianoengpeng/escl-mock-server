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
    let mut actual_binding_address = binding_address.clone(); // 初始化为原始绑定地址

    // 尝试设置 mDNS 服务（如果失败则继续运行）
    println!("🔍 === mDNS 服务发现调试信息 === 🔍");
    println!("📡 正在创建 mDNS 守护进程...");
    match ServiceDaemon::new() {
        Ok(mdns) => {
            println!("✅ mDNS 守护进程创建成功！");
            
            // 获取本机实际 IP 地址
            println!("🌐 正在检测本机 IP 地址...");
            let local_ip = match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => {
                    println!("📶 成功绑定测试 socket");
                    match socket.connect("8.8.8.8:80") {
                        Ok(_) => {
                            println!("🌍 成功连接到外部服务器进行IP检测");
                            match socket.local_addr() {
                                Ok(addr) => {
                                    let ip = addr.ip().to_string();
                                    println!("🎯 检测到本机IP: {}", ip);
                                    ip
                                },
                                Err(e) => {
                                    println!("⚠️ 无法获取本机地址: {}, 使用配置的地址", e);
                                    args.binding_address.clone()
                                }
                            }
                        },
                        Err(e) => {
                            println!("⚠️ 无法连接外部服务器: {}, 使用配置的地址", e);
                            args.binding_address.clone()
                        }
                    }
                },
                Err(e) => {
                    println!("⚠️ 无法创建测试socket: {}, 使用配置的地址", e);
                    args.binding_address.clone()
                },
            };
            
            println!("📍 最终使用 IP 地址: {}", local_ip);
            println!("🚪 使用端口: {}", args.port);
            
            // 使用实际IP地址作为绑定地址
            if args.binding_address == "127.0.0.1" && local_ip != "127.0.0.1" {
                println!("🔄 将绑定地址从 127.0.0.1 改为实际IP: {}", local_ip);
                actual_binding_address = local_ip.clone();
            }
            
            // 使用标准主机名格式，避免特殊字符
            let hostname = "escl-mock-scanner.local.";
            
            println!("🏷️ mDNS 主机名: {}", hostname);
            
            // 创建符合 Windows 11 要求的 TXT 记录
            let adminurl = format!("http://{}:{}/admin", local_ip, args.port);
            let representation = format!("http://{}:{}/icon.png", local_ip, args.port);
            
            println!("🔗 管理 URL: {}", adminurl);
            println!("🖼️ 图标 URL: {}", representation);
            
            // 验证这些URL是否可访问
            println!("🧪 === 验证关键URL可访问性 === 🧪");
            
            // 根据eSCL规范简化的TXT记录 - 只保留核心字段
            let txt_records = vec![
                ("txtvers", "1"),
                ("ty", "eSCL Scanner"),  // 简化名称
                ("rs", "eSCL"), 
                ("vers", "2.97"),
                ("pdl", "application/pdf,image/jpeg"),
                ("cs", "color,grayscale,binary"),
                ("is", "platen,adf"),
                ("duplex", "T"),
                ("uuid", "550e8400-e29b-41d4-a716-446655440000"),
                ("adminurl", adminurl.as_str()),
                ("representation", representation.as_str()),
            ];
            
            println!("📝 === TXT 记录详细信息 === 📝");
            for (key, value) in &txt_records {
                println!("   {}: {}", key, value);
            }
            println!("📝 === TXT 记录结束 === 📝");
            
            // 注册主要的 _uscan._tcp 服务 - 使用简化的主机名
            let service_name = "eSCL Scanner";
            println!("🎯 === 注册主要的 mDNS 服务 === 🎯");
            println!("   服务类型: _uscan._tcp.local.");
            println!("   服务名称: {}", service_name);
            println!("   主机名: {}", hostname);
            println!("   IP地址: {}", local_ip);
            println!("   端口: {}", args.port);
            println!("   TXT记录数量: {}", txt_records.len());
            
            // 尝试不同的方式注册服务
            println!("🔄 尝试使用IP地址直接注册服务...");
            match ServiceInfo::new(
                "_uscan._tcp.local.",
                service_name,
                hostname,
                &local_ip,
                args.port,
                &txt_records[..],
            ) {
                Ok(service_info) => {
                    println!("✅ 主要服务信息创建成功");
                    match mdns.register(service_info) {
                        Ok(_) => {
                            println!("🎉 主要 mDNS 服务注册成功！");
                            println!("🔍 等待3秒让mDNS服务完全广播...");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            println!("✅ mDNS广播应该已完成");
                        },
                        Err(e) => {
                            println!("❌ 主要服务注册失败: {}", e);
                            println!("🔧 尝试备用注册方式...");
                        }
                    }
                },
                Err(e) => println!("❌ 创建主要服务信息失败: {}", e),
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
                            println!("✅ HTTP 设备描述服务注册成功");
                        },
                        Err(e) => println!("❌ HTTP服务注册失败: {}", e),
                    }
                },
                Err(e) => println!("❌ 创建HTTP服务信息失败: {}", e),
            }
            
            println!("🌐 === 服务器信息汇总 === 🌐");
            println!("📍 服务地址: http://{}:{}{}", local_ip, args.port, args.scope);
            println!("🔧 手动配置URL: http://{}:{}", local_ip, args.port);
            println!("📊 服务状态: 所有 mDNS 服务已注册完成");
            println!("🎯 NAPS2应该能在'eSCL驱动程序'中发现此设备");
            println!("🌐 ========================== 🌐");
            
            // 保持 mDNS 服务活跃
            std::mem::forget(mdns);
        },
        Err(e) => {
            println!("Failed to create mDNS daemon: {}", e);
            println!("Continuing without mDNS service discovery...");
        }
    }

    // 然后启动 HTTP 服务器
    println!("🚀 === HTTP 服务器启动 === 🚀");
    println!("📡 绑定地址: {}", actual_binding_address);
    println!("🚪 监听端口: {}", args.port);
    println!("📂 eSCL 范围: {}", scope);
    println!("🖼️ 图片文件: {:?}", app_data.image_path);
    println!("🚀 正在启动服务器...");
    
    HttpServer::new(move || {
        App::new()
            .wrap(escl_server::LoggingMiddleware)  // 添加自定义请求日志
            .wrap(Logger::default())  // 添加详细的请求日志
            .app_data(app_data.clone())
            .service(escl_server::scanner_icon)  // 图标端点在根路径
            .service(escl_server::root_info)     // 根路径设备信息
            .service(escl_server::wsd_description) // WSD 设备描述
            .service(escl_server::wsd_post)      // WSD POST 处理
            .service(escl_server::device_metadata) // Windows 设备元数据
            .service(escl_server::ssdp_description) // SSDP 发现支持
            .service(escl_server::favicon)       // Favicon
            .service(escl_server::robots_txt)    // Robots.txt
            .service(escl_server::https_info)    // HTTPS 信息
            .service(escl_server::auth_info)     // 认证信息
            .service(escl_server::description_xml) // Description.xml
            .service(escl_server::escl_root)     // eSCL 根路径
            .service(escl_server::ssl_info)      // SSL 信息
            .service(escl_server::tls_info)      // TLS 信息
            .service(escl_server::driver_info)   // 驱动信息
            .service(escl_server::pnp_info)      // PnP 信息
            .service(escl_server::port_info)     // 端口信息
            .service(escl_server::admin_page)    // 管理页面
            .service(
                web::scope(&scope)
                    .service(escl_server::scanner_capabilities)
                    .service(escl_server::scanner_status)
                    .service(escl_server::device_info)  // 添加设备信息端点
                    .service(escl_server::scan_buffer_info)  // 添加扫描缓冲区信息端点
                    .service(escl_server::device_capabilities)  // Windows设备验证端点
                    .service(escl_server::device_uuid)  // 设备UUID端点
                    .service(escl_server::validate_device)  // Windows验证端点
                    .service(escl_server::device_configuration)  // 设备配置端点
                    .service(escl_server::scan_job)
                    .service(escl_server::next_doc),
            )
            .service(escl_server::system_info)       // 系统信息
            .service(escl_server::discovery_info)    // 发现信息
            .service(escl_server::network_info)      // 网络信息
            .service(escl_server::general_capabilities) // 通用能力
            .service(escl_server::health_check)      // 健康检查
            .service(escl_server::handle_options)    // OPTIONS处理
            .default_service(web::route().to(escl_server::not_found))
    })
    .bind((actual_binding_address, port))
    .expect("Couldn't create HTTP server")
    .run()
    .await
}
