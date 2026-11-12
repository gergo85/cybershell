//! Az exe ikonjának beágyazása (assets/cybershell.ico, a forrása assets/cybershell.svg).
fn main() {
    println!("cargo:rerun-if-changed=assets/cybershell.rc");
    println!("cargo:rerun-if-changed=assets/cybershell.ico");
    embed_resource::compile("assets/cybershell.rc", embed_resource::NONE).manifest_optional().unwrap();
}
