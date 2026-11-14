// The carved pumpkin's view blur, gone when the hat is hidden.
fn smys_pumpkin(value: hud_overlays::CameraOverlays, game: &crate::game::Game<'_>) -> hud_overlays::CameraOverlays {
    let mut value = value;
    if !crate::game::showmeyourskin::pumpkin_overlay(game) {
        value.equipment.retain(|t| t != "misc/pumpkinblur");
    }
    value
}
