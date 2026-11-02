use super::SHADER;

#[test]
fn the_shader_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(SHADER).expect("the shader parses");
    let flags = naga::valid::ValidationFlags::all();
    naga::valid::Validator::new(flags, naga::valid::Capabilities::default())
        .validate(&module)
        .expect("the shader validates");
}
