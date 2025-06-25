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
use chrono::Local;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse, Transform},
    Error, Result
};
use futures::future::LocalBoxFuture;
use std::future::{Ready, ready};

// 智能获取服务器IP地址的辅助函数
fn get_server_address(req: &HttpRequest) -> (String, String) {
    // 首先尝试从Host头获取
    if let Some(host_header) = req.headers().get("Host").and_then(|h| h.to_str().ok()) {
        // 分离IP和端口
        if let Some(colon_pos) = host_header.rfind(':') {
            let ip = &host_header[..colon_pos];
            let port = &host_header[colon_pos + 1..];
            return (ip.to_string(), port.to_string());
        } else {
            return (host_header.to_string(), "8080".to_string());
        }
    }
    
    // 如果没有Host头，尝试从连接信息获取
    let connection_info = req.connection_info();
    let host = connection_info.host();
    
    if let Some(colon_pos) = host.rfind(':') {
        let ip = &host[..colon_pos];
        let port = &host[colon_pos + 1..];
        (ip.to_string(), port.to_string())
    } else {
        (host.to_string(), "8080".to_string())
    }
}

// 获取完整的服务器URL前缀
fn get_server_url_prefix(req: &HttpRequest) -> String {
    let (ip, port) = get_server_address(req);
    let scheme = if req.connection_info().scheme() == "https" { "https" } else { "http" };
    format!("{}://{}:{}", scheme, ip, port)
}

// 全局请求记录中间件
pub struct LoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for LoggingMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = LoggingMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LoggingMiddlewareService { service }))
    }
}

pub struct LoggingMiddlewareService<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for LoggingMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let timestamp = Local::now().format("%H:%M:%S%.3f");
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let path = req.path().to_string();
        let peer_addr = req.peer_addr();
        
        // 获取User-Agent用于识别客户端类型
        let user_agent = req.headers()
            .get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("Unknown")
            .to_string();
        
        let host = req.headers()
            .get("Host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("Unknown")
            .to_string();
            
        println!("\n🌐 === [{timestamp}] 新的HTTP请求 === 🌐");
        println!("📡 方法: {method}");
        println!("🔗 URI: {uri}");
        println!("📍 路径: {path}");
        if let Some(peer) = peer_addr {
            println!("👤 客户端IP: {peer}");
        }
        println!("🏠 主机头: {host}");
        println!("🖥️ 客户端: {user_agent}");
        
        // 标记特殊的客户端
        if user_agent.contains("NAPS2") {
            println!("🎯 检测到 NAPS2 扫描软件！");
        } else if user_agent.contains("WSD") {
            println!("🔍 检测到 Windows 设备发现请求！");
        } else if user_agent.contains("Microsoft") {
            println!("🪟 检测到 Microsoft 相关请求！");
        }
        
        // 标记重要的eSCL端点
        let endpoint_type = match path.as_str() {
            p if p.contains("ScannerCapabilities") => "📋 扫描仪能力查询",
            p if p.contains("ScanJobs") => "📤 扫描任务操作",
            p if p.contains("ScannerStatus") => "💡 扫描仪状态查询", 
            p if p.contains("NextDocument") => "📥 获取扫描文档",
            p if p.contains("icon") => "🖼️ 设备图标请求",
            "/" => "🏠 根目录访问",
            _ => "❓ 其他请求",
        };
        println!("🎯 请求类型: {endpoint_type}");
        
        println!("🌐 ================================= 🌐\n");

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            let status = res.status();
            
            println!("\n📤 === [{timestamp}] 响应发送 === 📤");
            println!("📊 状态码: {status}");
            
            if status.is_success() {
                println!("✅ 请求处理成功");
            } else if status == StatusCode::NOT_FOUND {
                println!("⚠️ 404 - 端点未找到！可能是客户端探测");
            } else {
                println!("❌ 请求处理异常");
            }
            
            println!("📤 ========================== 📤\n");
            
            Ok(res)
        })
    }
}

// 添加详细的请求日志记录函数
fn log_request_details(req: &HttpRequest, endpoint_name: &str) {
    println!("\n🔵 === {} REQUEST === 🔵", endpoint_name);
    println!("📍 Method: {}", req.method());
    println!("📍 URI: {}", req.uri());
    println!("📍 Path: {}", req.path());
    println!("📍 Query: {:?}", req.query_string());
    println!("📍 Version: {:?}", req.version());
    
    println!("📍 Headers:");
    for (name, value) in req.headers().iter() {
        println!("     {}: {:?}", name, value);
    }
    
    if let Some(peer) = req.peer_addr() {
        println!("📍 Client IP: {}", peer);
    }
    
    println!("🔵 === END {} === 🔵\n", endpoint_name);
}

// 添加请求体日志记录函数
fn log_request_body(body: &web::Bytes, endpoint_name: &str) {
    if !body.is_empty() {
        println!("📝 === {} REQUEST BODY === 📝", endpoint_name);
        if let Ok(body_str) = std::str::from_utf8(body) {
            println!("{}", body_str);
        } else {
            println!("Binary data ({} bytes)", body.len());
        }
        println!("📝 === END BODY === 📝\n");
    }
}

#[get("/ScannerCapabilities")]
async fn scanner_capabilities(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    log_request_details(&req, "ScannerCapabilities");

    println!("ScannerCaps downloaded");

    // 获取主机信息以动态替换URL
    let host = req.headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    
    let scheme = if req.connection_info().scheme() == "https" { "https" } else { "http" };
    let admin_uri = format!("{}://{}/admin", scheme, host);
    let icon_uri = format!("{}://{}/icon.png", scheme, host);
    
    // 动态替换URL
    let scanner_caps = data.scanner_caps
        .replace("DYNAMIC_ADMIN_URI", &admin_uri)
        .replace("DYNAMIC_ICON_URI", &icon_uri);

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(scanner_caps)
}

#[get("/ScannerStatus")]
async fn scanner_status(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "ScannerStatus");

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

#[get("/icon.png")]
async fn scanner_icon(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "ScannerIcon");

    println!("Scanner icon requested");
    
    // 返回一个简单的 1x1 像素透明 PNG
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0B, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
    ];

    HttpResponse::build(StatusCode::OK)
        .content_type("image/png")
        .body(png_data)
}

#[post("/ScanJobs")]
async fn scan_job(req: HttpRequest, body: web::Bytes, data: web::Data<AppState>) -> impl Responder {
    log_request_details(&req, "ScanJobs");
    log_request_body(&body, "ScanJobs");
    
    let full_url = req.full_url();
    let generated_uuid = Uuid::new_v4();
    
    // 尝试解析扫描请求以确定扫描源
    let scan_source = if let Ok(body_str) = std::str::from_utf8(&body) {
        println!("🔍 Analyzing scan request body...");
        if body_str.contains("<scan:InputSource>Adf</scan:InputSource>") 
           || body_str.contains("Feeder") 
           || body_str.contains("ADF") {
            println!("✅ Detected ADF scan request");
            ScanSource::Adf
        } else {
            println!("✅ Detected Platen scan request");
            ScanSource::Platen
        }
    } else {
        println!("⚠️ Could not parse scan request, defaulting to Platen");
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

// 添加 ScanBufferInfo 端点 - Windows 11 第三阶段验证必需
#[actix_web::route("/ScanBufferInfo", method = "PUT")]
async fn scan_buffer_info(req: HttpRequest, body: web::Bytes, _data: web::Data<AppState>) -> impl Responder {
    log_request_details(&req, "ScanBufferInfo");
    log_request_body(&body, "ScanBufferInfo");
    
    println!("📋 ScanBufferInfo validation request received");
    
    // 解析扫描设置以进行验证
    let body_str = String::from_utf8_lossy(&body);
    
    // 基本验证 - 检查是否包含必要的设置
    let has_valid_settings = body_str.contains("<scan:ScanSettings") 
        || body_str.contains("InputSource") 
        || body_str.contains("ColorMode")
        || body_str.contains("XResolution");
    
    if !has_valid_settings {
        println!("❌ Invalid scan settings provided");
        return HttpResponse::build(StatusCode::CONFLICT)
            .content_type("text/xml")
            .body(r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ClientErrorDetails xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    <scan:ClientError>InvalidScanTicket</scan:ClientError>
</scan:ClientErrorDetails>"#);
    }
    
    println!("✅ Scan settings validated successfully");
    
    // 返回扫描缓冲区信息
    let scan_buffer_info = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScanBufferInfo xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03" 
                     xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
    <scan:ImageWidth>2550</scan:ImageWidth>
    <scan:ImageHeight>3300</scan:ImageHeight>
    <scan:BytesPerLine>7650</scan:BytesPerLine>
    <scan:BytesRequired>25245000</scan:BytesRequired>
    <scan:InputSourceType>Platen</scan:InputSourceType>
    <scan:ColorMode>RGB24</scan:ColorMode>
    <scan:XResolution>300</scan:XResolution>
    <scan:YResolution>300</scan:YResolution>
</scan:ScanBufferInfo>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(scan_buffer_info)
}

// 添加Windows设备验证端点
#[actix_web::route("/eSCL/DeviceCapabilities", method = "GET")]
async fn device_capabilities(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceCapabilities");
    println!("Windows device capabilities requested");
    
    // 重定向到标准的ScannerCapabilities
    HttpResponse::MovedPermanently()
        .insert_header(("Location", "/eSCL/ScannerCapabilities"))
        .finish()
}

// 添加Windows可能需要的设备识别端点
#[get("/eSCL/DeviceUUID")]
async fn device_uuid(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceUUID");
    println!("Device UUID requested");
    
    let uuid_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:DeviceUUID xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    550e8400-e29b-41d4-a716-446655440000
</scan:DeviceUUID>"#;

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(uuid_xml)
}

// 添加Windows设备添加时的验证端点
#[actix_web::route("/eSCL/Validate", method = "POST")]
async fn validate_device(req: HttpRequest, body: web::Bytes) -> impl Responder {
    log_request_details(&req, "ValidateDevice");
    log_request_body(&body, "ValidateDevice");
    
    println!("🔍 Windows device validation request received");
    
    // Windows可能发送验证请求来确认设备兼容性
    let validation_response = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ValidationResponse xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    <scan:Valid>true</scan:Valid>
    <scan:SupportedVersion>2.97</scan:SupportedVersion>
    <scan:DeviceReady>true</scan:DeviceReady>
</scan:ValidationResponse>"#;

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(validation_response)
}

// 添加Windows可能需要的设备配置端点
#[get("/eSCL/Configuration")]
async fn device_configuration(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceConfiguration");
    println!("Device configuration requested");
    
    let config_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:DeviceConfiguration xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    <scan:DeviceSettings>
        <scan:AutoPowerOff>false</scan:AutoPowerOff>
        <scan:PowerSaveMode>false</scan:PowerSaveMode>
        <scan:NetworkSettings>
            <scan:IPAddress>192.168.44.128</scan:IPAddress>
            <scan:Port>8080</scan:Port>
            <scan:Protocol>HTTP</scan:Protocol>
        </scan:NetworkSettings>
    </scan:DeviceSettings>
</scan:DeviceConfiguration>"#;

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(config_xml)
}

#[get("/ScanJobs/{uuid}/NextDocument")]
async fn next_doc(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    log_request_details(&req, "NextDocument");
    
    let full_url = req.full_url();

    println!("📄 Document is requested (UUID: {})", path.as_str());
    let mut data_guard = data.scan_jobs.lock().await;
    let uuid = &Uuid::from_str(&path.into_inner()).unwrap();

    // 获取扫描源信息
    let scan_source = {
        let sources_guard = data.scan_sources.lock().await;
        sources_guard.get(uuid).cloned().unwrap_or(ScanSource::Platen)
    };

    // 获取或创建扫描任务，并检查页面限制
    let (current_page, max_pages, scan_source_type) = match data_guard.get_mut(uuid) {
        None => {
            // 新的扫描任务 - 准备返回第一页
            let max_pages = match scan_source {
                ScanSource::Platen => 1,
                ScanSource::Adf => 5,  // ADF模拟最多5页
            };
            data_guard.insert(*uuid, ScanJob { 
                retrieved_pages: 1,  // 即将返回第一页
                scan_source: scan_source.clone(),
                max_pages,
            });
            (1, max_pages, scan_source)
        }
        Some(job) => {
            // 计算下一页的页码
            let next_page = job.retrieved_pages + 1;
            
            // 检查是否超出页面限制
            if next_page > job.max_pages {
                println!("No more pages available for {:?} source (requested page {} of {})", 
                        job.scan_source, next_page, job.max_pages);
                return HttpResponse::NotFound().finish();
            }
            
            // 更新页面计数
            job.retrieved_pages = next_page;
            (next_page, job.max_pages, job.scan_source.clone())
        }
    };

    println!("Serving page {} of {} for {:?} source", current_page, max_pages, scan_source_type);

    // 尝试使用指定的图片文件，如果失败则回退到内置图片
    if let Some(image_path) = &data.image_path {
        // 如果路径以 / 开头，尝试转换为相对路径
        let corrected_path = if image_path.starts_with("/res/") {
            image_path.strip_prefix("/").unwrap_or(image_path)
        } else if image_path.starts_with("\\res\\") {
            image_path.strip_prefix("\\").unwrap_or(image_path)
        } else {
            image_path
        };
        
        match tokio::fs::File::open(corrected_path).await {
            Ok(file) => {
                println!("Using custom image from: {}", corrected_path);
                let stream = ReaderStream::new(file);
                return HttpResponse::Ok()
                    .content_type("image/jpeg")
                    .insert_header((header::CONTENT_LOCATION, format!("{full_url}")))
                    .streaming(stream);
            }
            Err(e) => {
                println!("Failed to open custom image '{}': {}. Trying original path...", corrected_path, e);
                
                // 如果修正的路径也失败，尝试原始路径
                if corrected_path != image_path {
                    match tokio::fs::File::open(image_path).await {
                        Ok(file) => {
                            println!("Using custom image from original path: {}", image_path);
                            let stream = ReaderStream::new(file);
                            return HttpResponse::Ok()
                                .content_type("image/jpeg")
                                .insert_header((header::CONTENT_LOCATION, format!("{full_url}")))
                                .streaming(stream);
                        }
                        Err(e2) => {
                            println!("Also failed to open original path '{}': {}. Using default image.", image_path, e2);
                        }
                    }
                } else {
                    println!("Using default image.");
                }
                // 继续执行，使用默认图片
            }
        }
    }
    
    // 使用内置的默认图片
    println!("Using default embedded image");
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .insert_header((header::CONTENT_LOCATION, format!("{full_url}")))
        .body(&include_bytes!("../res/example_image.jpg")[..])
}

// 添加设备信息端点 - Windows 11 可能需要
#[get("/DeviceInfo")]
async fn device_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceInfo");

    println!("DeviceInfo requested");

    // 获取服务器地址信息
    let (server_ip, _) = get_server_address(&req);
    let url_prefix = get_server_url_prefix(&req);
    let device_uri = format!("{}/eSCL", url_prefix);
    let admin_uri = format!("{}/admin", url_prefix);

    let icon_uri = format!("{}/icon.png", url_prefix);
    
    let device_info_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:DeviceInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                 xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03"
                 xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm"
                 xsi:schemaLocation="http://schemas.hp.com/imaging/escl/2011/05/03 eSCL.xsd">
    <pwg:MakeAndModel>eSCL Scanner</pwg:MakeAndModel>
    <pwg:SerialNumber>ESC-MOCK-001</pwg:SerialNumber>
    <scan:UUID>550e8400-e29b-41d4-a716-446655440000</scan:UUID>
    <scan:DeviceURI>{}</scan:DeviceURI>
    <scan:AdminURI>{}</scan:AdminURI>
    <scan:IconURI>{}</scan:IconURI>
    <scan:Manufacturer>MockCompany</scan:Manufacturer>
    <scan:ModelName>eSCL Scanner</scan:ModelName>
    <scan:ModelNumber>v2024</scan:ModelNumber>
    <scan:FirmwareVersion>1.0.0</scan:FirmwareVersion>
    <scan:DeviceCategory>Scanner</scan:DeviceCategory>
    <scan:NetworkProtocol>HTTP</scan:NetworkProtocol>
    <scan:IPAddress>{}</scan:IPAddress>
    <scan:MACAddress>00:11:22:33:44:55</scan:MACAddress>
</scan:DeviceInfo>"#, device_uri, admin_uri, icon_uri, server_ip);

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(device_info_xml)
}

// 添加根路径端点
#[get("/")]
async fn root_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "RootInfo");

    println!("Root info requested");
    
    // 获取服务器地址信息
    let url_prefix = get_server_url_prefix(&req);
    
    let root_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
    <specVersion>
        <major>1</major>
        <minor>0</minor>
    </specVersion>
    <device>
        <deviceType>urn:schemas-upnp-org:device:Scanner:1</deviceType>
        <friendlyName>eSCL Scanner</friendlyName>
        <manufacturer>MockCompany</manufacturer>
        <manufacturerURL>http://www.mockcompany.com</manufacturerURL>
        <modelDescription>eSCL网络扫描仪</modelDescription>
        <modelName>eSCL Scanner</modelName>
        <modelNumber>v2024</modelNumber>
        <modelURL>{}/admin</modelURL>
        <serialNumber>ESC-MOCK-001</serialNumber>
        <UDN>uuid:550e8400-e29b-41d4-a716-446655440000</UDN>
        <presentationURL>{}/admin</presentationURL>
        <iconList>
            <icon>
                <mimetype>image/png</mimetype>
                <width>32</width>
                <height>32</height>
                <depth>8</depth>
                <url>/icon.png</url>
            </icon>
        </iconList>
        <serviceList>
            <service>
                <serviceType>urn:schemas-hp-com:service:imaging:ScanService:1</serviceType>
                <serviceId>urn:schemas-hp-com:serviceId:ScanService</serviceId>
                <SCPDURL>/eSCL/ScannerCapabilities</SCPDURL>
                <controlURL>/eSCL</controlURL>
                <eventSubURL>/eSCL</eventSubURL>
            </service>
        </serviceList>
    </device>
</root>"#, url_prefix, url_prefix);

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(root_xml)
}

// 改进的 WSD 设备描述端点
#[get("/wsd")]
async fn wsd_description(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "WSDDescription");

    println!("WSD description requested");
    
    // 获取客户端IP用于响应
    let host = req.headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    
    let wsd_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope 
    xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
    xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery"
    xmlns:wsdp="http://schemas.xmlsoap.org/ws/2006/02/devprof"
    xmlns:pnpx="http://schemas.microsoft.com/windows/pnpx/2005/10"
    xmlns:tns="http://schemas.microsoft.com/windows/2007/08/devicefoundation">
    <soap:Header>
        <wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</wsa:Action>
        <wsa:MessageID>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:MessageID>
        <wsa:RelatesTo>urn:uuid:550e8400-e29b-41d4-a716-446655440001</wsa:RelatesTo>
    </soap:Header>
    <soap:Body>
        <wsd:ProbeMatches>
            <wsd:ProbeMatch>
                <wsa:EndpointReference>
                    <wsa:Address>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:Address>
                </wsa:EndpointReference>
                <wsd:Types>wsdp:Device pnpx:NetworkDevice scan:Scanner</wsd:Types>
                <wsd:Scopes>
                    http://schemas.xmlsoap.org/ws/2005/04/discovery/ldap
                    http://schemas.microsoft.com/windows/pnpx/2005/10/category/scanner
                    http://schemas.microsoft.com/windows/pnpx/2005/10/category/imaging
                </wsd:Scopes>
                <wsd:XAddrs>http://{}/wsd</wsd:XAddrs>
                <wsd:MetadataVersion>1</wsd:MetadataVersion>
            </wsd:ProbeMatch>
        </wsd:ProbeMatches>
    </soap:Body>
</soap:Envelope>"#, host);

    HttpResponse::build(StatusCode::OK)
        .content_type("application/soap+xml; charset=utf-8")
        .insert_header(("Cache-Control", "no-cache"))
        .body(wsd_xml)
}

// Windows可能会查询的WS-Discovery端点
#[post("/wsd")]
async fn wsd_post(req: HttpRequest, body: web::Bytes) -> impl Responder {
    log_request_details(&req, "WSD_POST");
    log_request_body(&body, "WSD_POST");
    
    println!("WS-Discovery POST request received");
    
    // 解析请求以确定响应类型
    let body_str = String::from_utf8_lossy(&body);
    let host = req.headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    
    let response = if body_str.contains("GetMetadataRequest") {
        // 响应GetMetadata请求
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
               xmlns:wsx="http://schemas.xmlsoap.org/ws/2004/09/mex"
               xmlns:wsdp="http://schemas.xmlsoap.org/ws/2006/02/devprof"
               xmlns:pnpx="http://schemas.microsoft.com/windows/pnpx/2005/10"
               xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    <soap:Header>
        <wsa:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2004/09/mex/GetMetadataResponse</wsa:Action>
        <wsa:MessageID>urn:uuid:550e8400-e29b-41d4-a716-446655440002</wsa:MessageID>
        <wsa:RelatesTo>urn:uuid:550e8400-e29b-41d4-a716-446655440003</wsa:RelatesTo>
    </soap:Header>
    <soap:Body>
        <wsx:Metadata>
            <wsx:MetadataSection Dialect="http://schemas.xmlsoap.org/ws/2006/02/devprof/ThisModel">
                <wsdp:ThisModel>
                    <wsdp:Manufacturer>MockCompany</wsdp:Manufacturer>
                    <wsdp:ManufacturerUrl>http://www.mockcompany.com</wsdp:ManufacturerUrl>
                    <wsdp:ModelName>eSCL Mock Scanner</wsdp:ModelName>
                    <wsdp:ModelNumber>1.0</wsdp:ModelNumber>
                    <wsdp:ModelUrl>http://www.mockcompany.com</wsdp:ModelUrl>
                    <wsdp:PresentationUrl>http://{}/</wsdp:PresentationUrl>
                </wsdp:ThisModel>
            </wsx:MetadataSection>
            <wsx:MetadataSection Dialect="http://schemas.xmlsoap.org/ws/2006/02/devprof/ThisDevice">
                <wsdp:ThisDevice>
                    <wsdp:FriendlyName>eSCL Mock Scanner</wsdp:FriendlyName>
                    <wsdp:FirmwareVersion>1.0.0</wsdp:FirmwareVersion>
                    <wsdp:SerialNumber>550e8400-e29b-41d4-a716-446655440000</wsdp:SerialNumber>
                </wsdp:ThisDevice>
            </wsx:MetadataSection>
            <wsx:MetadataSection Dialect="http://schemas.xmlsoap.org/ws/2006/02/devprof/Relationship">
                <wsdp:Relationship Type="http://schemas.xmlsoap.org/ws/2006/02/devprof/host">
                    <wsdp:Hosted>
                        <wsa:EndpointReference>
                            <wsa:Address>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:Address>
                        </wsa:EndpointReference>
                        <wsdp:Types>scan:ScannerServiceType</wsdp:Types>
                        <wsdp:ServiceId>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsdp:ServiceId>
                    </wsdp:Hosted>
                </wsdp:Relationship>
            </wsx:MetadataSection>
            <wsx:MetadataSection Dialect="pnpx:DeviceCategory">
                <pnpx:DeviceCategory>Scanner Imaging Device</pnpx:DeviceCategory>
            </wsx:MetadataSection>
        </wsx:Metadata>
    </soap:Body>
</soap:Envelope>"#, host)
    } else if body_str.contains("ScanAvailableEvent") {
        // 响应扫描可用事件订阅
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
               xmlns:wse="http://schemas.xmlsoap.org/ws/2004/08/eventing"
               xmlns:scan="http://schemas.microsoft.com/windows/2006/08/wdp/scan">
    <soap:Header>
        <wsa:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2004/08/eventing/SubscribeResponse</wsa:Action>
        <wsa:MessageID>urn:uuid:550e8400-e29b-41d4-a716-446655440004</wsa:MessageID>
        <wsa:RelatesTo>urn:uuid:550e8400-e29b-41d4-a716-446655440005</wsa:RelatesTo>
    </soap:Header>
    <soap:Body>
        <wse:SubscribeResponse>
            <wse:SubscriptionManager>
                <wsa:Address>http://{}/wsd/subscription</wsa:Address>
            </wse:SubscriptionManager>
            <wse:Expires>P0Y0M0DT0H5M0S</wse:Expires>
        </wse:SubscribeResponse>
    </soap:Body>
</soap:Envelope>"#, host)
    } else {
        // 默认响应
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
               xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
    <soap:Header>
        <wsa:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Hello</wsa:Action>
        <wsa:MessageID>urn:uuid:550e8400-e29b-41d4-a716-446655440006</wsa:MessageID>
    </soap:Header>
    <soap:Body>
        <wsd:Hello>
            <wsa:EndpointReference>
                <wsa:Address>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:Address>
            </wsa:EndpointReference>
            <wsd:Types>wsdp:Device pnpx:NetworkDevice</wsd:Types>
            <wsd:XAddrs>http://{}/wsd</wsd:XAddrs>
            <wsd:MetadataVersion>1</wsd:MetadataVersion>
        </wsd:Hello>
    </soap:Body>
</soap:Envelope>"#, host)
    };

    HttpResponse::Ok()
        .content_type("application/soap+xml; charset=utf-8")
        .insert_header(("Cache-Control", "no-cache"))
        .body(response)
}

// 添加 Windows 设备元数据端点
#[get("/device.xml")]
async fn device_metadata(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceMetadata");

    println!("Device metadata requested");
    
    // 获取服务器地址信息
    let (server_ip, _) = get_server_address(&req);
    
    let device_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" 
      xmlns:pnpx="http://schemas.microsoft.com/windows/pnpx/2005/10" 
      xmlns:df="http://schemas.microsoft.com/windows/2008/09/devicefoundation">
    <specVersion>
        <major>1</major>
        <minor>0</minor>
    </specVersion>
    <device>
        <pnpx:X_deviceCategory>Scanners</pnpx:X_deviceCategory>
        <pnpx:X_hardwareId>PnPX_ServiceId:550e8400-e29b-41d4-a716-446655440000</pnpx:X_hardwareId>
        <pnpx:X_compatibleId>PnPX_ServiceId:eSCL_Scanner</pnpx:X_compatibleId>
        <pnpx:X_physicalLocation></pnpx:X_physicalLocation>
        <pnpx:X_networkInterfaceLuid>0</pnpx:X_networkInterfaceLuid>
        <pnpx:X_ipAddress>{}</pnpx:X_ipAddress>
        <pnpx:X_ipVersion>4</pnpx:X_ipVersion>
        <df:X_deviceCategory>Multimedia.Scanner</df:X_deviceCategory>
        <deviceType>urn:schemas-upnp-org:device:Scanner:1</deviceType>
        <friendlyName>eSCL Mock Scanner</friendlyName>
        <manufacturer>MockCompany</manufacturer>
        <manufacturerURL>http://www.mockcompany.com</manufacturerURL>
        <modelDescription>eSCL Mock Scanner for Testing</modelDescription>
        <modelName>eSCL-Mock-Scanner</modelName>
        <modelNumber>1.0</modelNumber>
        <modelURL>http://www.mockcompany.com</modelURL>
        <serialNumber>550e8400-e29b-41d4-a716-446655440000</serialNumber>
        <UDN>uuid:550e8400-e29b-41d4-a716-446655440000</UDN>
        <iconList>
            <icon>
                <mimetype>image/png</mimetype>
                <width>32</width>
                <height>32</height>
                <depth>8</depth>
                <url>/icon.png</url>
            </icon>
        </iconList>
        <serviceList>
            <service>
                <serviceType>urn:schemas-hp-com:service:imaging:ScanService:1</serviceType>
                <serviceId>urn:schemas-hp-com:serviceId:ScanService</serviceId>
                <SCPDURL>/eSCL/ScannerCapabilities</SCPDURL>
                <controlURL>/eSCL</controlURL>
                <eventSubURL>/eSCL</eventSubURL>
            </service>
        </serviceList>
        <presentationURL>/</presentationURL>
    </device>
</root>"#, server_ip);

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(device_xml)
}

// 添加 SSDP 发现支持
#[get("/ssdp")]
async fn ssdp_description(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "SSDPDescription");

    println!("SSDP description requested");
    
    let url_prefix = get_server_url_prefix(&req);
    
    let ssdp_response = format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         DATE: {}\r\n\
         EXT:\r\n\
         LOCATION: {}/device.xml\r\n\
         SERVER: Windows/10 UPnP/1.0 eSCL-Mock-Server/1.0\r\n\
         ST: urn:schemas-upnp-org:device:Scanner:1\r\n\
         USN: uuid:550e8400-e29b-41d4-a716-446655440000::urn:schemas-upnp-org:device:Scanner:1\r\n\
         BOOTID.UPNP.ORG: 1\r\n\
         CONFIGID.UPNP.ORG: 1\r\n\r\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        url_prefix
    );

    HttpResponse::build(StatusCode::OK)
        .content_type("text/plain")
        .body(ssdp_response)
}

pub(crate) async fn not_found(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "⚠️ NOT_FOUND");

    println!("❌ MISSING ENDPOINT: {} {} (This endpoint might be needed by Windows!)", 
             req.method(), req.uri());
    
    // 提供一些建议的端点
    let suggested_endpoints = vec![
        "/eSCL/ScannerCapabilities",
        "/eSCL/ScannerStatus", 
        "/eSCL/ScanJobs",
        "/eSCL/DeviceInfo",
        "/icon.png",
        "/device.xml",
        "/wsd",
        "/ssdp",
        "/"
    ];
    
    let suggestion = format!(
        "❌ Endpoint not found: {} {}\n\n✅ Available endpoints:\n{}",
        req.method(),
        req.path(),
        suggested_endpoints.join("\n")
    );
    
    HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("text/plain")
        .body(suggestion)
}

// 添加一些Windows可能需要的其他端点

#[get("/favicon.ico")]
async fn favicon(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "Favicon");
    println!("Favicon requested - returning empty response");
    HttpResponse::NotFound().finish()
}

#[get("/robots.txt")]
async fn robots_txt(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "RobotsTxt");
    println!("Robots.txt requested");
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("User-agent: *\nDisallow: /")
}

// Windows可能会查询的根目录下的XML文件
#[get("/description.xml")]
async fn description_xml(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DescriptionXML");
    println!("Description.xml requested");
    
    // 重定向到device.xml
    HttpResponse::MovedPermanently()
        .insert_header(("Location", "/device.xml"))
        .finish()
}

// 添加Windows扫描仪安装相关的端点

#[get("/eSCL")]
async fn escl_root(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "eSCL_Root");
    println!("eSCL root endpoint requested - redirecting to capabilities");
    
    // 重定向到ScannerCapabilities
    HttpResponse::MovedPermanently()
        .insert_header(("Location", "/eSCL/ScannerCapabilities"))
        .finish()
}

// Windows可能检查的SSL/TLS信息
#[get("/ssl")]
async fn ssl_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "SSL_Info");
    println!("SSL info requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"ssl_enabled": false, "message": "HTTP only mock server"}"#)
}

#[get("/tls")]
async fn tls_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "TLS_Info");
    println!("TLS info requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"tls_enabled": false, "message": "HTTP only mock server"}"#)
}

// Windows设备驱动信息
#[get("/driver")]
async fn driver_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "Driver_Info");
    println!("Driver info requested");
    
    let driver_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<DriverInfo>
    <Manufacturer>MockCompany</Manufacturer>
    <Model>eSCL Mock Scanner</Model>
    <DriverVersion>1.0.0</DriverVersion>
    <eSCLVersion>2.63</eSCLVersion>
    <SupportedProtocols>
        <Protocol>HTTP</Protocol>
        <Protocol>eSCL</Protocol>
    </SupportedProtocols>
</DriverInfo>"#;

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(driver_xml)
}

// 可能的Windows PnP查询
#[get("/pnp")]
async fn pnp_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "PnP_Info");
    println!("PnP info requested");
    
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("PnP ID: MockCompany_eSCL_Scanner")
}

// Windows可能查询的端口信息
#[get("/port")]
async fn port_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "Port_Info");
    println!("Port info requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"http_port": 8080, "https_port": "not_supported"}"#)
}

// HTTPS重定向处理
#[get("/https")]
async fn https_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "HTTPS_Info");
    println!("HTTPS endpoint requested");
    
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("This mock server runs on HTTP only. HTTPS not supported.")
}

// 可能的认证端点
#[get("/auth")]
async fn auth_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "Auth");
    println!("Auth endpoint requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"auth_required": false, "method": "none"}"#)
}

// 添加Windows 11可能需要的额外端点

// Windows可能查询的系统信息
#[get("/system")]
async fn system_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "SystemInfo");
    println!("System info requested");
    
    let system_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<SystemInfo>
    <OSVersion>Mock OS 1.0</OSVersion>
    <FirmwareVersion>1.0.0</FirmwareVersion>
    <SystemUptime>86400</SystemUptime>
    <MemoryUsage>50</MemoryUsage>
    <NetworkStatus>Connected</NetworkStatus>
</SystemInfo>"#;

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(system_xml)
}

// Windows设备发现时的辅助端点
#[get("/discovery")]
async fn discovery_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DiscoveryInfo");
    println!("Discovery info requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"device_type": "scanner", "protocol": "eSCL", "version": "2.97"}"#)
}

// Windows可能查询的网络配置
#[get("/network")]
async fn network_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "NetworkInfo");
    println!("Network info requested");
    
    let host = req.headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    
    let network_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<NetworkInfo>
    <HostAddress>{}</HostAddress>
    <Protocol>HTTP</Protocol>
    <ConnectionType>Ethernet</ConnectionType>
    <IPv4Enabled>true</IPv4Enabled>
    <IPv6Enabled>false</IPv6Enabled>
    <DHCPEnabled>true</DHCPEnabled>
</NetworkInfo>"#, host);

    HttpResponse::Ok()
        .content_type("text/xml")
        .body(network_xml)
}

// Windows可能验证的功能端点
#[get("/capabilities")]
async fn general_capabilities(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "GeneralCapabilities");
    println!("General capabilities requested");
    
    // 重定向到标准的ScannerCapabilities
    HttpResponse::MovedPermanently()
        .insert_header(("Location", "/eSCL/ScannerCapabilities"))
        .finish()
}

// Windows可能需要的状态检查端点
#[get("/health")]
async fn health_check(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "HealthCheck");
    println!("Health check requested");
    
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status": "healthy", "uptime": 86400, "scanner_ready": true}"#)
}

// 处理可能的OPTIONS预检请求
#[actix_web::route("/{path:.*}", method = "OPTIONS")]
async fn handle_options(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "OPTIONS_Request");
    println!("OPTIONS preflight request");
    
    HttpResponse::Ok()
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS"))
        .insert_header(("Access-Control-Allow-Headers", "Content-Type, Authorization"))
        .insert_header(("Access-Control-Max-Age", "86400"))
        .finish()
}

// 添加管理页面端点 - mDNS adminurl指向的页面
#[get("/admin")]
async fn admin_page(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "AdminPage");
    println!("Admin page requested");
    
    let (server_ip, server_port) = get_server_address(&req);
    
    let admin_html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>eSCL Scanner - Administration</title>
    <meta charset="UTF-8">
</head>
<body>
    <h1>eSCL Scanner</h1>
    <h2>Device Administration</h2>
    <p><strong>Status:</strong> Ready</p>
    <p><strong>IP Address:</strong> {}</p>
    <p><strong>Port:</strong> {}</p>
    <p><strong>eSCL Version:</strong> 2.97</p>
    <p><strong>Supported Features:</strong></p>
    <ul>
        <li>Flatbed (Platen) scanning</li>
        <li>ADF (Auto Document Feeder) scanning</li>
        <li>Duplex scanning</li>
        <li>Color, Grayscale, Binary modes</li>
        <li>PDF and JPEG output formats</li>
    </ul>
    <p><strong>eSCL Endpoints:</strong></p>
    <ul>
        <li><a href="/eSCL/ScannerCapabilities">Scanner Capabilities</a></li>
        <li><a href="/eSCL/ScannerStatus">Scanner Status</a></li>
        <li><a href="/device.xml">Device Description</a></li>
    </ul>
</body>
</html>"#, server_ip, server_port);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(admin_html)
}
