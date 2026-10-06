use bo2_assets::{
    captured_frame::load_captured_frame, full_scene::load_full_scene, shader_bank::load_shader_bank,
};
use sha2::{Digest, Sha256};

/// Explicit opt-in local-original admission, kept distinct from synthetic tests.
#[test]
#[ignore = "requires three external manifest paths; contains no original fixture"]
fn external_world_dispatch_admission() {
    let frame = std::env::var("BO2_CAPTURED_FRAME").expect("external captured manifest");
    let scene_path = std::env::var("BO2_CAPTURED_SCENE").expect("external scene manifest");
    let bank_path = std::env::var("BO2_CAPTURED_BANK").expect("external native shader bank");
    let scene = load_full_scene(std::path::Path::new(&scene_path)).expect("full scene admission");
    let bank = load_shader_bank(std::path::Path::new(&bank_path)).expect("native bank admission");
    let bank_hash = format!("{:x}", Sha256::digest(std::fs::read(bank_path).unwrap()));
    let frame = load_captured_frame(std::path::Path::new(&frame), &scene, &bank, &bank_hash)
        .expect("captured frame admission");
    eprintln!(
        "captured admission: {} receipts, {} world draws",
        frame.manifest.draws.len(),
        frame.draws.len()
    );
    assert!(!frame.draws.is_empty());
}
