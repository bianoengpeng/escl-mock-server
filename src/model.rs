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

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub(crate) enum ScanSource {
    Platen,  // 平板
    Adf,     // 自动输稿器
}

impl Default for ScanSource {
    fn default() -> Self {
        ScanSource::Platen
    }
}

pub(crate) struct ScanJob {
    pub retrieved_pages: u32,
    pub scan_source: ScanSource,
    pub max_pages: u32,
}

impl Default for ScanJob {
    fn default() -> Self {
        ScanJob { 
            retrieved_pages: 0,
            scan_source: ScanSource::Platen,
            max_pages: 1,  // 平板默认只有1页
        }
    }
}

impl Display for ScanJob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "retrieved_pages = {}, source = {:?}, max_pages = {}", 
               self.retrieved_pages, self.scan_source, self.max_pages)
    }
}
