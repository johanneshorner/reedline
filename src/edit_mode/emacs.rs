use crate::{
    edit_mode::{
        keybindings::{
            add_common_control_bindings, add_common_edit_bindings, add_common_navigation_bindings,
            add_common_selection_bindings, edit_bind, KeyCombination, Keybindings,
        },
        EditMode,
    },
    enums::{EditCommand, ReedlineEvent, ReedlineRawEvent},
    PromptEditMode,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::keybindings::{KeyNode, PartialKeySequence};

/// Returns the current default emacs keybindings
pub fn default_emacs_keybindings() -> Keybindings {
    use EditCommand as EC;
    use KeyCode as KC;
    use KeyModifiers as KM;

    let mut kb = Keybindings::new();
    add_common_control_bindings(&mut kb);
    add_common_navigation_bindings(&mut kb);
    add_common_edit_bindings(&mut kb);
    add_common_selection_bindings(&mut kb);

    // This could be in common, but in Vi it also changes the mode
    kb.add_binding(
        KeyCombination {
            modifier: KM::NONE,
            key_code: KC::Enter,
        },
        vec![],
        ReedlineEvent::Enter,
    );

    // *** CTRL ***
    // Moves
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('b'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![ReedlineEvent::MenuLeft, ReedlineEvent::Left]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('f'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            ReedlineEvent::MenuRight,
            ReedlineEvent::Right,
        ]),
    );
    // Undo/Redo
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('g'),
        },
        vec![],
        edit_bind(EC::Redo),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('z'),
        },
        vec![],
        edit_bind(EC::Undo),
    );
    // Cutting
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('y'),
        },
        vec![],
        edit_bind(EC::PasteCutBufferBefore),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('w'),
        },
        vec![],
        edit_bind(EC::CutWordLeft),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('k'),
        },
        vec![],
        edit_bind(EC::CutToLineEnd),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('u'),
        },
        vec![],
        edit_bind(EC::CutFromStart),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('d'),
        },
        vec![],
        edit_bind(EC::CutWordRight),
    );
    // Edits
    kb.add_binding(
        KeyCombination {
            modifier: KM::CONTROL,
            key_code: KC::Char('t'),
        },
        vec![],
        edit_bind(EC::SwapGraphemes),
    );

    // *** ALT ***
    // Moves
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Left,
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Right,
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintWordComplete,
            edit_bind(EC::MoveWordRight { select: false }),
        ]),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('b'),
        },
        vec![],
        edit_bind(EC::MoveWordLeft { select: false }),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('f'),
        },
        vec![],
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintWordComplete,
            edit_bind(EC::MoveWordRight { select: false }),
        ]),
    );
    // Edits
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Delete,
        },
        vec![],
        edit_bind(EC::DeleteWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Backspace,
        },
        vec![],
        edit_bind(EC::BackspaceWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('m'),
        },
        vec![],
        ReedlineEvent::Edit(vec![EditCommand::BackspaceWord]),
    );
    // Case changes
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('u'),
        },
        vec![],
        edit_bind(EC::UppercaseWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('l'),
        },
        vec![],
        edit_bind(EC::LowercaseWord),
    );
    kb.add_binding(
        KeyCombination {
            modifier: KM::ALT,
            key_code: KC::Char('c'),
        },
        vec![],
        edit_bind(EC::CapitalizeChar),
    );

    kb
}

/// This parses the incoming Events like a emacs style-editor
pub struct Emacs {
    keybindings: Keybindings,
    partial_key_sequence: Option<PartialKeySequence>,
}

impl Default for Emacs {
    fn default() -> Self {
        Emacs {
            keybindings: default_emacs_keybindings(),
            partial_key_sequence: None,
        }
    }
}

impl Emacs {
    /// Emacs style input parsing constructor if you want to use custom keybindings
    pub const fn new(keybindings: Keybindings) -> Self {
        Emacs {
            keybindings,
            partial_key_sequence: None,
        }
    }

    fn handle_binding(
        &mut self,
        modifier: KeyModifiers,
        key_code: KeyCode,
    ) -> Option<ReedlineEvent> {
        let Some(mut partial_key_sequence) = self.partial_key_sequence.take() else {
            return match self.keybindings.find_binding(modifier, key_code)? {
                KeyNode::Sequence(sequence) => {
                    self.partial_key_sequence = Some(PartialKeySequence {
                        sequence,
                        history: if let KeyCode::Char(c) = key_code {
                            vec![if modifier == KeyModifiers::SHIFT {
                                c.to_ascii_uppercase()
                            } else {
                                c
                            }]
                        } else {
                            vec![]
                        },
                    });
                    Some(ReedlineEvent::None)
                }
                KeyNode::Event(reedline_event) => Some(reedline_event),
            };
        };

        if let KeyCode::Char(c) = key_code {
            partial_key_sequence
                .history
                .push(if modifier == KeyModifiers::SHIFT {
                    c.to_ascii_uppercase()
                } else {
                    c
                })
        }

        match partial_key_sequence
            .sequence
            .map
            .remove(&KeyCombination { modifier, key_code })
        {
            Some(KeyNode::Event(reedline_event)) => Some(reedline_event),
            Some(KeyNode::Sequence(sequence)) => {
                self.partial_key_sequence = Some(PartialKeySequence {
                    sequence,
                    history: partial_key_sequence.history,
                });
                Some(ReedlineEvent::None)
            }
            None => Some(ReedlineEvent::Edit(
                partial_key_sequence
                    .history
                    .into_iter()
                    .map(EditCommand::InsertChar)
                    .collect(),
            )),
        }
    }
}

impl EditMode for Emacs {
    fn parse_event(&mut self, event: ReedlineRawEvent) -> ReedlineEvent {
        match event.into() {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (modifiers, code) {
                (modifier, KeyCode::Char(c)) => {
                    // Note. The modifier can also be a combination of modifiers, for
                    // example:
                    //     KeyModifiers::CONTROL | KeyModifiers::ALT
                    //     KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
                    //
                    // Mixed modifiers are used by non american keyboards that have extra
                    // keys like 'alt gr'. Keep this in mind if in the future there are
                    // cases where an event is not being captured
                    let c = match modifier {
                        KeyModifiers::NONE => c,
                        _ => c.to_ascii_lowercase(),
                    };

                    self.handle_binding(modifier, KeyCode::Char(c))
                        .unwrap_or_else(|| {
                            if modifier == KeyModifiers::NONE
                                || modifier == KeyModifiers::SHIFT
                                || modifier == KeyModifiers::CONTROL | KeyModifiers::ALT
                                || modifier
                                    == KeyModifiers::CONTROL
                                        | KeyModifiers::ALT
                                        | KeyModifiers::SHIFT
                            {
                                ReedlineEvent::Edit(vec![EditCommand::InsertChar(
                                    if modifier == KeyModifiers::SHIFT {
                                        c.to_ascii_uppercase()
                                    } else {
                                        c
                                    },
                                )])
                            } else {
                                ReedlineEvent::None
                            }
                        })
                }
                _ => self
                    .handle_binding(modifiers, code)
                    .unwrap_or(ReedlineEvent::None),
            },

            Event::Mouse(_) => ReedlineEvent::Mouse,
            Event::Resize(width, height) => ReedlineEvent::Resize(width, height),
            Event::FocusGained => ReedlineEvent::None,
            Event::FocusLost => ReedlineEvent::None,
            Event::Paste(body) => ReedlineEvent::Edit(vec![EditCommand::InsertString(
                body.replace("\r\n", "\n").replace('\r', "\n"),
            )]),
        }
    }

    fn edit_mode(&self) -> PromptEditMode {
        PromptEditMode::Emacs
    }
}

#[cfg(test)]
mod test {
    use crate::edit_mode::keybindings::KeyCombination;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ctrl_l_leads_to_clear_screen_event() {
        let mut emacs = Emacs::default();
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::ClearScreen);
    }

    #[test]
    fn overriding_default_keybindings_works() {
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyCombination {
                modifier: KeyModifiers::CONTROL,
                key_code: KeyCode::Char('l'),
            },
            vec![],
            ReedlineEvent::HistoryHintComplete,
        );

        let mut emacs = Emacs::new(keybindings);
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::HistoryHintComplete);
    }

    #[test]
    fn inserting_character_works() {
        let mut emacs = Emacs::default();
        let l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::NONE,
        )))
        .unwrap();
        let result = emacs.parse_event(l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('l')])
        );
    }

    #[test]
    fn inserting_capital_character_works() {
        let mut emacs = Emacs::default();

        let uppercase_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::SHIFT,
        )))
        .unwrap();
        let result = emacs.parse_event(uppercase_l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('L')])
        );
    }

    #[test]
    fn return_none_reedline_event_when_keybinding_is_not_found() {
        let keybindings = Keybindings::default();

        let mut emacs = Emacs::new(keybindings);
        let ctrl_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let result = emacs.parse_event(ctrl_l);

        assert_eq!(result, ReedlineEvent::None);
    }

    #[test]
    fn inserting_capital_character_for_non_ascii_remains_as_is() {
        let mut emacs = Emacs::default();

        let uppercase_l = ReedlineRawEvent::try_from(Event::Key(KeyEvent::new(
            KeyCode::Char('😀'),
            KeyModifiers::SHIFT,
        )))
        .unwrap();
        let result = emacs.parse_event(uppercase_l);

        assert_eq!(
            result,
            ReedlineEvent::Edit(vec![EditCommand::InsertChar('😀')])
        );
    }
}
