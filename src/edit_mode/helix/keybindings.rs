use crate::edit_mode::{
    keybindings::{
        add_common_control_bindings, add_common_edit_bindings, add_common_navigation_bindings,
        add_common_selection_bindings,
    },
    Keybindings,
};

/// Default Helix normal mode keybindings
pub fn default_helix_normal_keybindings() -> Keybindings {
    let mut kb = Keybindings::new();

    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    kb
}

/// Default Helix insert keybindings
pub fn default_helix_insert_keybindings() -> Keybindings {
    let mut kb = Keybindings::new();

    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_edit_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    kb
}
