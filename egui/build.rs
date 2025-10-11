fn main() {
    let res = embed_resource::compile("resources.rc", embed_resource::NONE);
    res.manifest_optional().unwrap();
}
