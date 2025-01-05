use {
    crate::{enums::ReedlineEvent, EditCommand},
    crossterm::event::{KeyCode, KeyModifiers},
    serde::{Deserialize, Serialize},
    std::collections::{hash_map::Entry, HashMap},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct KeyCombination {
    pub modifier: KeyModifiers,
    pub key_code: KeyCode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum KeyNode {
    Sequence(Sequence),
    Event(ReedlineEvent),
}

impl KeyNode {
    fn new(key_combinations: Vec<KeyCombination>, command: ReedlineEvent) -> Self {
        let mut prev = KeyNode::Event(command);
        for key_combination in key_combinations.into_iter().rev() {
            prev = KeyNode::Sequence(Sequence {
                map: HashMap::from([(key_combination, prev)]),
            });
        }
        prev
    }

    // TODO needs a test
    fn merge(&mut self, other: Self) {
        match (&mut *self, other) {
            (KeyNode::Sequence(sequence), KeyNode::Sequence(mut other_sequence)) => {
                for (k, v) in sequence.map.iter_mut() {
                    if let Some(o) = other_sequence.map.remove(k) {
                        v.merge(o);
                    }
                }
                sequence.map.extend(other_sequence.map.drain());
            }
            (_, other @ _) => *self = other,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Sequence {
    // name: Option<String>,
    map: HashMap<KeyCombination, KeyNode>,
}

/// Main definition of editor keybindings
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Keybindings {
    /// Defines a keybinding for a reedline event
    pub bindings: Sequence,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self::new()
    }
}

impl Keybindings {
    /// New keybining
    pub fn new() -> Self {
        Self {
            bindings: Sequence::default(),
        }
    }

    /// Defines an empty keybinding object
    pub fn empty() -> Self {
        Self::new()
    }

    /// Adds a keybinding
    ///
    /// # Panics
    ///
    /// If `key_combinations` is empty
    ///
    /// or
    ///
    /// If `command` is an empty [`ReedlineEvent::UntilFound`]
    pub fn add_binding(
        &mut self,
        start_key_combination: KeyCombination,
        key_combinations: Vec<KeyCombination>,
        command: ReedlineEvent,
    ) {
        if let ReedlineEvent::UntilFound(subcommands) = &command {
            assert!(
                !subcommands.is_empty(),
                "UntilFound should contain a series of potential events to handle"
            );
        }

        let key_node = KeyNode::new(key_combinations, command);
        match self.bindings.map.entry(start_key_combination) {
            Entry::Occupied(mut occupied_entry) => occupied_entry.get_mut().merge(key_node),
            Entry::Vacant(vacant_entry) => _ = vacant_entry.insert(key_node),
        }
    }

    /// Find a keybinding based on the modifier and keycode
    pub fn find_binding(&self, modifier: KeyModifiers, key_code: KeyCode) -> Option<ReedlineEvent> {
        let key_combo = KeyCombination { modifier, key_code };
        match self.bindings.map.get(&key_combo) {
            Some(KeyNode::Event(event)) => Some(event.clone()),
            Some(_) => todo!(),
            None => None,
        }
    }

    /// Remove a keybinding
    ///
    /// Returns `Some(ReedlineEvent)` if the key combination was previously bound to a particular [`ReedlineEvent`]
    pub fn remove_binding(
        &mut self,
        _start_key_combination: KeyCombination,
        _key_combinations: Vec<KeyCombination>,
    ) -> Option<ReedlineEvent> {
        todo!()
    }

    /// Get assigned keybindings
    pub fn get_keybindings(&self) -> &HashMap<KeyCombination, ReedlineEvent> {
        todo!()
    }
}

pub fn edit_bind(command: EditCommand) -> ReedlineEvent {
    ReedlineEvent::Edit(vec![command])
}

/// Add the basic special keybindings
///
/// `Ctrl-C`, `Ctrl-D`, `Ctrl-O`, `Ctrl-R`
/// + `Esc`
/// + `Ctrl-O` to open the external editor
pub fn add_common_control_bindings(kb: &mut Keybindings) {
    use KeyCode as KC;
    use KeyModifiers as KM;

    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Esc,
        },
        vec![],
        ReedlineEvent::Esc,
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('c'),
        },
        vec![],
        ReedlineEvent::CtrlC,
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('d'),
        },
        vec![],
        ReedlineEvent::CtrlD,
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('l'),
        },
        vec![],
        ReedlineEvent::ClearScreen,
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('r'),
        },
        vec![],
        ReedlineEvent::SearchHistory,
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('o'),
        },
        vec![],
        ReedlineEvent::OpenEditor,
    );
}
/// Add the arrow navigation and its `Ctrl` variants
pub fn add_common_navigation_bindings(kb: &mut Keybindings) {
    use EditCommand as EC;
    use KeyCode as KC;
    use KeyModifiers as KM;

    // Arrow keys without modifier
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Up,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuUp, ReedlineEvent::Up]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Down,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuDown, ReedlineEvent::Down]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Left,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuLeft, ReedlineEvent::Left]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Right,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            ReedlineEvent::MenuRight,
            ReedlineEvent::Right,
        ]),
    );

    // Ctrl Left and Right
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Left,
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Right,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintWordComplete,
            edit_bind(EC::MoveWordRight { select: false }),
        ]),
    );
    // Home/End & ctrl+a/ctrl+e
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Home,
        },
        vec![],
        edit_bind(EC::MoveToLineStart { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('a'),
        },
        vec![],
        edit_bind(EC::MoveToLineStart { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::End,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            edit_bind(EC::MoveToLineEnd { select: false }),
        ]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('e'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            edit_bind(EC::MoveToLineEnd { select: false }),
        ]),
    );
    // Ctrl Home/End
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Home,
        },
        vec![],
        edit_bind(EC::MoveToStart { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::End,
        },
        vec![],
        edit_bind(EC::MoveToEnd { select: false }),
    );
    // EMACS arrows
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('p'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuUp, ReedlineEvent::Up]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('n'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuDown, ReedlineEvent::Down]),
    );
}

/// Add basic functionality to edit
///
/// `Delete`, `Backspace` and the basic variants do delete words
pub fn add_common_edit_bindings(kb: &mut Keybindings) {
    use EditCommand as EC;
    use KeyCode as KC;
    use KeyModifiers as KM;
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Backspace,
        },
        vec![],
        edit_bind(EC::Backspace),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Delete,
        },
        vec![],
        edit_bind(EC::Delete),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Backspace,
        },
        vec![],
        edit_bind(EC::BackspaceWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Delete,
        },
        vec![],
        edit_bind(EC::DeleteWord),
    );
    // Base commands should not affect cut buffer
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('h'),
        },
        vec![],
        edit_bind(EC::Backspace),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('w'),
        },
        vec![],
        edit_bind(EC::BackspaceWord),
    );
    #[cfg(feature = "system_clipboard")]
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL | KM::SHIFT,
            key_code: KC::Char('x'),
        },
        vec![],
        edit_bind(EC::CutSelectionSystem),
    );
    #[cfg(feature = "system_clipboard")]
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL | KM::SHIFT,
            key_code: KC::Char('c'),
        },
        vec![],
        edit_bind(EC::CopySelectionSystem),
    );
    #[cfg(feature = "system_clipboard")]
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL | KM::SHIFT,
            key_code: KC::Char('v'),
        },
        vec![],
        edit_bind(EC::PasteSystem),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Enter,
        },
        vec![],
        edit_bind(EC::InsertNewline),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT,
            key_code: KC::Enter,
        },
        vec![],
        edit_bind(EC::InsertNewline),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('j'),
        },
        vec![],
        ReedlineEvent::Enter,
    );
}

pub fn add_common_selection_bindings(kb: &mut Keybindings) {
    use EditCommand as EC;
    use KeyCode as KC;
    use KeyModifiers as KM;

    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT,
            key_code: KC::Left,
        },
        vec![],
        edit_bind(EC::MoveLeft { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT,
            key_code: KC::Right,
        },
        vec![],
        edit_bind(EC::MoveRight { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT | KM::CONTROL,
            key_code: KC::Left,
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT | KM::CONTROL,
            key_code: KC::Right,
        },
        vec![],
        edit_bind(EC::MoveWordRight { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT,
            key_code: KC::End,
        },
        vec![],
        edit_bind(EC::MoveToLineEnd { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT | KM::CONTROL,
            key_code: KC::End,
        },
        vec![],
        edit_bind(EC::MoveToEnd { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT,
            key_code: KC::Home,
        },
        vec![],
        edit_bind(EC::MoveToLineStart { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::SHIFT | KM::CONTROL,
            key_code: KC::Home,
        },
        vec![],
        edit_bind(EC::MoveToStart { select: true }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL | KM::SHIFT,
            key_code: KC::Char('a'),
        },
        vec![],
        edit_bind(EC::SelectAll),
    );
}
