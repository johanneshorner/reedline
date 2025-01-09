use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    edit_mode::{
        keybindings::{
            add_common_control_bindings, add_common_edit_bindings, add_common_navigation_bindings,
            add_common_selection_bindings,
        },
        Keybindings,
    },
    HelixEvent, HelixNormal, KeyCombination, ReedlineEvent,
};

/// Default Helix normal mode keybindings
pub fn default_helix_normal_keybindings() -> Keybindings {
    let mut kb = Keybindings::new();

    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Esc,
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::NormalMode),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('i'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::InsertMode)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('v'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::SelectMode)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('h'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveCharLeft)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('j'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveVisualLineDown)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('k'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveVisualLineUp)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('l'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveCharRight)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('w'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveNextWordStart)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('b'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MovePrevWordStart)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Char('e'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveNextWordEnd)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::SHIFT,
            key_code: KeyCode::Char('w'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveNextLongWordStart)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::SHIFT,
            key_code: KeyCode::Char('b'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MovePrevLongWordStart)),
    );

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::SHIFT,
            key_code: KeyCode::Char('e'),
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::Normal(HelixNormal::MoveNextLongWordEnd)),
    );

    kb
}

/// Default Helix insert keybindings
pub fn default_helix_insert_keybindings() -> Keybindings {
    let mut kb = Keybindings::new();

    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_edit_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    kb.add_binding(
        KeyCombination {
            modifier: KeyModifiers::NONE,
            key_code: KeyCode::Esc,
        },
        vec![],
        ReedlineEvent::Helix(HelixEvent::NormalMode),
    );

    kb
}
