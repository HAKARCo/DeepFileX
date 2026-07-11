#![allow(clippy::collapsible_if)]

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        if std::env::var("DISABLE_MANIFEST").is_err() {
            let is_debug = std::env::var("PROFILE").map(|v| v == "debug").unwrap_or(false);
            let level = if is_debug { "asInvoker" } else { "requireAdministrator" };
            let mut res = winres::WindowsResource::new();
            let manifest_src = format!(
                r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="{}" uiAccess="false"/>
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
                "#,
                level
            );
            res.set_manifest(&manifest_src);
            res.set_icon("src/deepfilex.ico");
            res.compile().unwrap();
        }
    }
}
