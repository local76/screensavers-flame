use std::path::Path;
use library::core::build_resources::{
    write_brand_rc, DEFAULT_COMPANY_NAME, DEFAULT_LEGAL_COPYRIGHT, DEFAULT_PRODUCT_NAME,
};

fn main() {
    let ico = "assets/scene-flame.ico";
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }
    if !Path::new(ico).exists() { return; }

    let pkg = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    let rc = write_brand_rc(
        "build/windows_resource.rc",
        ico,
        &pkg,
        DEFAULT_PRODUCT_NAME,
        DEFAULT_COMPANY_NAME,
        DEFAULT_LEGAL_COPYRIGHT,
    );
    embed_resource::compile(&rc, embed_resource::NONE);
}