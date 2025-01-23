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
use regex::Regex;
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

const IPV4_REGEX: &str = include_str!("../res/regexes/IPV4_REGEX");
const IPV6_REGEX: &str = include_str!("../res/regexes/IPV6_REGEX");

fn validate_addr(args: &Cli) {
    let ipv4_regex = Regex::new(IPV4_REGEX);
    let ipv6_regex = Regex::new(IPV6_REGEX);

    let is_ipv4 = ipv4_regex.unwrap().is_match(&args.binding_address);
    let is_ipv6 = ipv6_regex.unwrap().is_match(&args.binding_address);

    if !(is_ipv4 || is_ipv6) {
        Cli::command()
            .error(ErrorKind::ValueValidation, "Invalid address")
            .exit()
    }
}

pub(crate) fn parse_cli() -> Cli {
    let args = Cli::parse();

    validate_addr(&args);

    args
}
