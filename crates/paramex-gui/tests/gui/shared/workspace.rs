use paramex_gui::state::Workspace;

#[test]
fn default_workspace_is_transfer() {
    assert_eq!(Workspace::default(), Workspace::Transfer);
}

#[test]
fn index_round_trips() {
    assert_eq!(Workspace::Transfer.index(), 0);
    assert_eq!(Workspace::Tlm.index(), 1);
    assert_eq!(Workspace::from_index(0), Workspace::Transfer);
    assert_eq!(Workspace::from_index(1), Workspace::Tlm);
    assert_eq!(Workspace::from_index(99), Workspace::Transfer);
}
