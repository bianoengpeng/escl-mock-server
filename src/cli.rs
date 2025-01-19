use std::fmt::{Display, Formatter};
use clap::error::ErrorKind;
use clap::{arg, CommandFactory, Parser};
use regex::Regex;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
#[command(about, version)]
pub(crate) struct Cli {
    #[arg(short = 'a', long = "bindaddr", default_value = "127.0.0.1")]
    pub(crate) binding_address: String,
    #[arg(short = 'c', long = "scannercaps")]
    pub(crate) scanner_caps_file: Option<String>,
    #[arg(short = 'i', long = "image")]
    pub(crate) served_image: Option<String>,
    #[arg(value_parser = clap::value_parser!(u16).range(1..), short = 'p', long = "port", default_value = "8080")]
    pub(crate) port: u16,
}

impl Display for Cli {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Binding to {}:{}", self.binding_address, self.port)
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
        Cli::command().error(
            ErrorKind::ValueValidation,
            "Invalid address"
        ).exit()
    }
}

pub(crate) fn parse_cli() -> Cli {
    let args = Cli::parse();

    validate_addr(&args);

    args
}