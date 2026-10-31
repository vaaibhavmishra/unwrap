use warpui::platform::WindowStyle;
use warpui::{App, SingletonEntity};

use super::{NewWorkspaceSource, RootView};
use crate::GlobalResourceHandles;
use crate::auth::AuthStateProvider;
use crate::workspace::WorkspaceRegistry;

#[test]
fn local_workspace_opens_without_an_account() {
    App::test((), |mut app| async move {
        crate::workspace::view::tests::initialize_app(&mut app);
        app.update(|ctx| {
            let auth_state = AuthStateProvider::as_ref(ctx).get();
            auth_state.set_user(None);
        });

        let global_resource_handles = GlobalResourceHandles::mock(&mut app);
        let (window_id, root_view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            RootView::new(
                global_resource_handles,
                NewWorkspaceSource::Empty {
                    previous_active_window: None,
                    shell: None,
                },
                ctx,
            )
        });

        app.read(|ctx| {
            let workspace = root_view.as_ref(ctx).workspace_view().unwrap();
            assert_eq!(
                WorkspaceRegistry::as_ref(ctx)
                    .get(window_id, ctx)
                    .unwrap()
                    .id(),
                workspace.id(),
            );
        });
    });
}
