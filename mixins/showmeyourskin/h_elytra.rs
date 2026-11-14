// A player's elytra: hidden, or drawn without its glint.
fn smys_elytra(
    game: &crate::game::Game<'_>,
    mob: &horizon_client::mobs::Mob,
    chest: Option<&str>,
    camera: glam::DVec3,
    partial: f32,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> ::core::ops::ControlFlow<()> {
    crate::game::showmeyourskin::elytra(game, mob, chest, camera, partial, light, out)
}
