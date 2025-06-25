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

// 添加详细的请求日志记录函数
fn log_request_details(req: &HttpRequest, endpoint_name: &str) {
    println!("\n=== {} REQUEST ===", endpoint_name);
    println!("Method: {}", req.method());
    println!("URI: {}", req.uri());
    println!("Path: {}", req.path());
    println!("Query: {:?}", req.query_string());
    println!("Version: {:?}", req.version());
    
    println!("Headers:");
    for (name, value) in req.headers().iter() {
        println!("  {}: {:?}", name, value);
    }
    
    if let Some(peer) = req.peer_addr() {
        println!("Peer Address: {}", peer);
    }
    
    println!("=== END REQUEST ===\n");
}

#[get("/ScannerCapabilities")]
async fn scanner_capabilities(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    log_request_details(&req, "ScannerCapabilities");

    println!("ScannerCaps downloaded");

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(data.scanner_caps.to_owned())
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

// 添加设备信息端点 - Windows 11 可能需要
#[get("/DeviceInfo")]
async fn device_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceInfo");

    println!("DeviceInfo requested");

    let device_info_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:DeviceInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                 xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03"
                 xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm"
                 xsi:schemaLocation="http://schemas.hp.com/imaging/escl/2011/05/03 eSCL.xsd">
    <pwg:MakeAndModel>MockCompany eSCL Mock Scanner</pwg:MakeAndModel>
    <pwg:SerialNumber>550e8400-e29b-41d4-a716-446655440000</pwg:SerialNumber>
    <scan:UUID>550e8400-e29b-41d4-a716-446655440000</scan:UUID>
    <scan:DeviceURI>http://192.168.44.128:8000/eSCL</scan:DeviceURI>
</scan:DeviceInfo>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(device_info_xml)
}

// 添加根路径端点
#[get("/")]
async fn root_info(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "RootInfo");

    println!("Root info requested");
    
    let root_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
    <specVersion>
        <major>1</major>
        <minor>0</minor>
    </specVersion>
    <device>
        <deviceType>urn:schemas-upnp-org:device:Scanner:1</deviceType>
        <friendlyName>eSCL Mock Scanner</friendlyName>
        <manufacturer>MockCompany</manufacturer>
        <manufacturerURL>http://www.mockcompany.com</manufacturerURL>
        <modelDescription>eSCL Mock Scanner</modelDescription>
        <modelName>eSCL-Mock-Scanner</modelName>
        <modelNumber>1.0</modelNumber>
        <modelURL>http://www.mockcompany.com</modelURL>
        <serialNumber>550e8400-e29b-41d4-a716-446655440000</serialNumber>
        <UDN>uuid:550e8400-e29b-41d4-a716-446655440000</UDN>
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
</root>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(root_xml)
}

// 添加 WSD 设备描述端点
#[get("/wsd")]
async fn wsd_description(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "WSDDescription");

    println!("WSD description requested");
    
    let wsd_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope 
    xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
    xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery"
    xmlns:wsdp="http://schemas.xmlsoap.org/ws/2006/02/devprof"
    xmlns:pnpx="http://schemas.microsoft.com/windows/pnpx/2005/10">
    <soap:Header>
        <wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
        <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</wsa:Action>
        <wsa:MessageID>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:MessageID>
    </soap:Header>
    <soap:Body>
        <wsd:ProbeMatches>
            <wsd:ProbeMatch>
                <wsa:EndpointReference>
                    <wsa:Address>urn:uuid:550e8400-e29b-41d4-a716-446655440000</wsa:Address>
                </wsa:EndpointReference>
                <wsd:Types>wsdp:Device pnpx:NetworkDevice</wsd:Types>
                <wsd:Scopes>
                    http://schemas.xmlsoap.org/ws/2005/04/discovery/ldap
                    http://schemas.microsoft.com/windows/pnpx/2005/10/category/scanner
                </wsd:Scopes>
                <wsd:XAddrs>http://192.168.44.128:8000/</wsd:XAddrs>
                <wsd:MetadataVersion>1</wsd:MetadataVersion>
            </wsd:ProbeMatch>
        </wsd:ProbeMatches>
    </soap:Body>
</soap:Envelope>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("application/soap+xml")
        .body(wsd_xml)
}

// 添加 Windows 设备元数据端点
#[get("/device.xml")]
async fn device_metadata(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "DeviceMetadata");

    println!("Device metadata requested");
    
    let device_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
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
        <pnpx:X_ipAddress>192.168.44.128</pnpx:X_ipAddress>
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
</root>"#;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/xml")
        .body(device_xml)
}

// 添加 SSDP 发现支持
#[get("/ssdp")]
async fn ssdp_description(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "SSDPDescription");

    println!("SSDP description requested");
    
    let ssdp_response = format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         DATE: {}\r\n\
         EXT:\r\n\
         LOCATION: http://192.168.44.128:8000/device.xml\r\n\
         SERVER: Windows/10 UPnP/1.0 eSCL-Mock-Server/1.0\r\n\
         ST: urn:schemas-upnp-org:device:Scanner:1\r\n\
         USN: uuid:550e8400-e29b-41d4-a716-446655440000::urn:schemas-upnp-org:device:Scanner:1\r\n\
         BOOTID.UPNP.ORG: 1\r\n\
         CONFIGID.UPNP.ORG: 1\r\n\r\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    HttpResponse::build(StatusCode::OK)
        .content_type("text/plain")
        .body(ssdp_response)
}

pub(crate) async fn not_found(req: HttpRequest) -> impl Responder {
    log_request_details(&req, "NotFound");

    println!(
        "The following path was accessed but is not implemented: {}",
        req.path()
    );

    HttpResponse::build(StatusCode::NOT_FOUND).body("Not found 404")
}
