//! Az exe ikonjának beágyazása (assets/cyberhub.ico, a forrása assets/cyberhub.svg).
fn main() {
    println!("cargo:rerun-if-changed=assets/cyberhub.rc");
    println!("cargo:rerun-if-changed=assets/cyberhub.ico");
    embed_resource::compile("assets/cyberhub.rc", embed_resource::NONE).manifest_optional().unwrap();
}
