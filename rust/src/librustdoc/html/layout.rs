// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::io;
use std::path::PathBuf;

use externalfiles::ExternalHtml;

#[derive(Clone)]
pub struct Layout {
    pub logo: String,
    pub favicon: String,
    pub external_html: ExternalHtml,
    pub krate: String,
}

pub struct Page<'a> {
    pub title: &'a str,
    pub css_class: &'a str,
    pub root_path: &'a str,
    pub description: &'a str,
    pub keywords: &'a str,
    pub resource_suffix: &'a str,
}

pub fn render<T: fmt::Display, S: fmt::Display>(
    dst: &mut io::Write, layout: &Layout, page: &Page, sidebar: &S, t: &T,
    css_file_extension: bool, themes: &[PathBuf])
    -> io::Result<()>
{
    write!(dst,
"<!DOCTYPE html>\
<html lang=\"en\">\
<head>\
    <meta charset=\"utf-8\">\
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\
    <meta name=\"generator\" content=\"rustdoc\">\
    <meta name=\"description\" content=\"{description}\">\
    <meta name=\"keywords\" content=\"{keywords}\">\
    <title>{title}</title>\
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}normalize{suffix}.css\">\
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}rustdoc{suffix}.css\" \
          id=\"mainThemeStyle\">\
    {themes}\
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}dark{suffix}.css\">\
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}light{suffix}.css\" \
          id=\"themeStyle\">\
    <script src=\"{root_path}storage{suffix}.js\"></script>\
    {css_extension}\
    {favicon}\
    {in_header}\
</head>\
<body class=\"rustdoc {css_class}\">\
    <!--[if lte IE 8]>\
    <div class=\"warning\">\
        This old browser is unsupported and will most likely display funky \
        things.\
    </div>\
    <![endif]-->\
    {before_content}\
    <nav class=\"sidebar\">\
        <div class=\"sidebar-menu\">&#9776;</div>\
        {logo}\
        {sidebar}\
    </nav>\
    <div class=\"theme-picker\">\
        <button id=\"theme-picker\" aria-label=\"Pick another theme!\">\
            <img src=\"{root_path}brush{suffix}.svg\" width=\"18\" alt=\"Pick another theme!\">\
        </button>\
        <div id=\"theme-choices\"></div>\
    </div>\
    <script src=\"{root_path}theme{suffix}.js\"></script>\
    <nav class=\"sub\">\
        <form class=\"search-form js-only\">\
            <div class=\"search-container\">\
                <input class=\"search-input\" name=\"search\" \
                       autocomplete=\"off\" \
                       placeholder=\"Click or press ‘S’ to search, ‘?’ for more options…\" \
                       type=\"search\">\
                <a id=\"settings-menu\" href=\"{root_path}settings.html\">\
                    <img src=\"{root_path}wheel{suffix}.svg\" width=\"18\" alt=\"Change settings\">\
                </a>\
            </div>\
        </form>\
    </nav>\
    <section id=\"main\" class=\"content\">{content}</section>\
    <section id=\"search\" class=\"content hidden\"></section>\
    <section class=\"footer\"></section>\
    <aside id=\"help\" class=\"hidden\">\
        <div>\
            <h1 class=\"hidden\">Help</h1>\
            <div class=\"shortcuts\">\
                <h2>Keyboard Shortcuts</h2>\
                <dl>\
                    <dt><kbd>?</kbd></dt>\
                    <dd>Show this help dialog</dd>\
                    <dt><kbd>S</kbd></dt>\
                    <dd>Focus the search field</dd>\
                    <dt><kbd>↑</kbd></dt>\
                    <dd>Move up in search results</dd>\
                    <dt><kbd>↓</kbd></dt>\
                    <dd>Move down in search results</dd>\
                    <dt><kbd>↹</kbd></dt>\
                    <dd>Switch tab</dd>\
                    <dt><kbd>&#9166;</kbd></dt>\
                    <dd>Go to active search result</dd>\
                    <dt><kbd>+</kbd></dt>\
                    <dd>Expand all sections</dd>\
                    <dt><kbd>-</kbd></dt>\
                    <dd>Collapse all sections</dd>\
                </dl>\
            </div>\
            <div class=\"infos\">\
                <h2>Search Tricks</h2>\
                <p>\
                    Prefix searches with a type followed by a colon (e.g. \
                    <code>fn:</code>) to restrict the search to a given type.\
                </p>\
                <p>\
                    Accepted types are: <code>fn</code>, <code>mod</code>, \
                    <code>struct</code>, <code>enum</code>, \
                    <code>trait</code>, <code>type</code>, <code>macro</code>, \
                    and <code>const</code>.\
                </p>\
                <p>\
                    Search functions by type signature (e.g. \
                    <code>vec -> usize</code> or <code>* -> vec</code>)\
                </p>\
                <p>\
                    Search multiple things at once by splitting your query with comma (e.g. \
                    <code>str,u8</code> or <code>String,struct:Vec,test</code>)\
                </p>\
            </div>\
        </div>\
    </aside>\
    {after_content}\
    <script>\
        window.rootPath = \"{root_path}\";\
        window.currentCrate = \"{krate}\";\
    </script>\
    <script src=\"{root_path}aliases.js\"></script>\
    <script src=\"{root_path}main{suffix}.js\"></script>\
    <script defer src=\"{root_path}search-index.js\"></script>\
</body>\
</html>",
    css_extension = if css_file_extension {
        format!("<link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}theme{suffix}.css\">",
                root_path = page.root_path,
                suffix=page.resource_suffix)
    } else {
        "".to_owned()
    },
    content   = *t,
    root_path = page.root_path,
    css_class = page.css_class,
    logo      = if layout.logo.is_empty() {
        "".to_string()
    } else {
        format!("<a href='{}{}/index.html'>\
                 <img src='{}' alt='logo' width='100'></a>",
                page.root_path, layout.krate,
                layout.logo)
    },
    title     = page.title,
    description = page.description,
    keywords = page.keywords,
    favicon   = if layout.favicon.is_empty() {
        "".to_string()
    } else {
        format!(r#"<link rel="shortcut icon" href="{}">"#, layout.favicon)
    },
    in_header = layout.external_html.in_header,
    before_content = layout.external_html.before_content,
    after_content = layout.external_html.after_content,
    sidebar   = *sidebar,
    krate     = layout.krate,
    themes = themes.iter()
                   .filter_map(|t| t.file_stem())
                   .filter_map(|t| t.to_str())
                   .map(|t| format!(r#"<link rel="stylesheet" type="text/css" href="{}{}{}.css">"#,
                                    page.root_path,
                                    t,
                                    page.resource_suffix))
                   .collect::<String>(),
    suffix=page.resource_suffix,
    )
}

pub fn redirect(dst: &mut io::Write, url: &str) -> io::Result<()> {
    // <script> triggers a redirect before refresh, so this is fine.
    write!(dst,
r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="refresh" content="0;URL={url}">
</head>
<body>
    <p>Redirecting to <a href="{url}">{url}</a>...</p>
    <script>location.replace("{url}" + location.search + location.hash);</script>
</body>
</html>"##,
    url = url,
    )
}
