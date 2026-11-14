// The carved pumpkin's view blur, gone when the hat is hidden.
// No closure: injection contexts can't hold closures, async, try/?, unsafe or const blocks.
fn smys_pumpkin(value: hud_overlays::CameraOverlays, game: &crate::game::Game<'_>) -> hud_overlays::CameraOverlays {
    let mut value = value;
    if !crate::game::showmeyourskin::pumpkin_overlay(game) {
        let mut i = 0;
        while i < value.equipment.len() {
            if value.equipment[i] == "misc/pumpkinblur" {
                value.equipment.remove(i);
            } else {
                i += 1;
            }
        }
    }
    value
}
