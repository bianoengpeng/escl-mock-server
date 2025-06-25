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

use clap::error::ErrorKind;
use clap::{arg, CommandFactory, Parser};
use std::fmt::{Display, Formatter};

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
#[command(about, version)]
pub(crate) struct Cli {
    #[arg(short = 'a', long = "bindaddr", default_value = "127.0.0.1")]
    pub(crate) binding_address: String,
    #[arg(short = 's', long = "scope", default_value = "/eSCL")]
    pub(crate) scope: String,
    #[arg(short = 'c', long = "scannercaps")]
    pub(crate) scanner_caps_file: Option<String>,
    #[arg(short = 'i', long = "image")]
    pub(crate) served_image: Option<String>,
    #[arg(value_parser = clap::value_parser!(u16).range(1..), short = 'p', long = "port", default_value = "8080")]
    pub(crate) port: u16,
}

impl Display for Cli {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Binding to {}:{}\nScope is: \"{}\"\nPossible URL: http://{0}:{1}/{2}",
            self.binding_address, self.port, self.scope
        )
    }
}



fn validate_addr(args: &Cli) {
    println!("Validating address: {}", args.binding_address);

    // 简化的地址验证 - 只检查基本格式
    use std::net::IpAddr;
    if args.binding_address.parse::<IpAddr>().is_err() {
        println!("Invalid IP address format: {}", args.binding_address);
        Cli::command()
            .error(ErrorKind::ValueValidation, "Invalid address")
            .exit()
    }

    println!("Address validation passed");
}

pub(crate) fn parse_cli() -> Cli {
    let args = Cli::parse();

    validate_addr(&args);

    args
}
