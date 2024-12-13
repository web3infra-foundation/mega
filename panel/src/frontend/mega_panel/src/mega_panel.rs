use crate::mega_panel_settings::{MegaPanelDockPosition, MegaPanelSettings};
use anyhow::{anyhow, Context};
use db::kvp::KEY_VALUE_STORE;
use fs::Fs;
use gpui::private::serde_derive::{Deserialize, Serialize};
use gpui::private::serde_json;
use gpui::{actions, div, Action, AppContext, AssetSource, AsyncWindowContext, Div, ElementId, EventEmitter, FocusHandle, FocusableView, FontWeight, InteractiveElement, IntoElement, Model, ParentElement, Pixels, PromptLevel, Render, SharedString, Stateful, Styled, Task, View, ViewContext, VisualContext, WeakView, WindowContext};
use mega::Mega;
use settings::Settings;
use std::sync::Arc;
use text::BufferId;
use util::{ResultExt, TryFutureExt};
use workspace::dock::{DockPosition, Panel, PanelEvent};
use workspace::ui::{
    h_flex, v_flex, Button, Clickable, Color, FixedWidth, IconName, IconPosition, Label,
    LabelCommon, LabelSize, StyledExt, StyledTypography,
};
use workspace::Workspace;
use worktree::{ProjectEntryId, WorktreeId};

mod mega_panel_settings;

const MEGA_PANEL_KEY: &str = "MegaPanel";

actions!(mega_panel, [ToggleFocus, ToggleFuseMount, CheckoutPath,]);

pub struct MegaPanel {
    mega_handle: Model<Mega>,
    // workspace: WeakView<Workspace>,
    focus_handle: FocusHandle,
    // scroll_handle: UniformListScrollHandle,
    fs: Arc<dyn Fs>,
    pending_serialization: Task<Option<()>>,
    width: Option<Pixels>,
}

#[derive(Serialize, Deserialize)]
struct SerializedMegaPanel {
    width: Option<Pixels>,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MegaEntry {
    Dir(WorktreeId, ProjectEntryId),
    File(WorktreeId, BufferId),
}

#[derive(Debug)]
pub enum Event {
    Focus,
}

pub fn init_settings(cx: &mut AppContext) {
    MegaPanelSettings::register(cx);
}

pub fn init(assets: impl AssetSource, cx: &mut AppContext) {
    init_settings(cx);
    file_icons::init(assets, cx);

    cx.observe_new_views(|workspace: &mut Workspace, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, cx| {
            workspace.toggle_panel_focus::<MegaPanel>(cx);
        });
    })
    .detach();
}

impl EventEmitter<Event> for MegaPanel {}

impl EventEmitter<PanelEvent> for MegaPanel {}

impl Render for MegaPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let mega_panel = v_flex()
            .id("mega_panel")
            .size_full()
            .relative()
            .on_action(cx.listener(Self::toggle_fuse_mount))
            .on_action(cx.listener(Self::checkout_path))
            .track_focus(&self.focus_handle)
            .gap_6()
            .p_4()
            .child(
                h_flex().justify_center().child(
                    Label::new("Mega Control Panel")
                        .single_line()
                        .weight(FontWeight::BOLD)
                        .size(LabelSize::Large),
                ),
            )
            .child(horizontal_separator(cx))
            .child(self.render_status(cx))
            .child(horizontal_separator(cx))
            .child(self.render_checkout_points(cx))
            .child(horizontal_separator(cx))
            .child(self.render_buttons(cx));

        mega_panel
    }
}

impl FocusableView for MegaPanel {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MegaPanel {
    fn persistent_name() -> &'static str {
        "Mega Panel"
    }

    fn position(&self, cx: &WindowContext) -> DockPosition {
        match MegaPanelSettings::get_global(cx).dock {
            MegaPanelDockPosition::Left => DockPosition::Left,
            MegaPanelDockPosition::Right => DockPosition::Right,
        }
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(&mut self, position: DockPosition, cx: &mut ViewContext<Self>) {
        settings::update_settings_file::<MegaPanelSettings>(
            self.fs.clone(),
            cx,
            move |settings, _| {
                let dock = match position {
                    DockPosition::Left | DockPosition::Bottom => MegaPanelDockPosition::Left,
                    DockPosition::Right => MegaPanelDockPosition::Right,
                };
                settings.dock = Some(dock);
            },
        );
    }

    fn size(&self, cx: &WindowContext) -> Pixels {
        self.width
            .unwrap_or_else(|| MegaPanelSettings::get_global(cx).default_width)
    }

    fn set_size(&mut self, size: Option<Pixels>, cx: &mut ViewContext<Self>) {
        self.width = size;
        self.serialize(cx);
        cx.notify();
    }

    fn icon(&self, cx: &WindowContext) -> Option<IconName> {
        MegaPanelSettings::get_global(cx)
            .button
            .then_some(IconName::FileGit)
    }

    fn icon_tooltip(&self, _cx: &WindowContext) -> Option<&'static str> {
        Some("Mega Panel")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleFocus)
    }
}

impl MegaPanel {
    pub async fn load(
        workspace: WeakView<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<View<Self>> {
        let serialized_panel = cx
            .background_executor()
            .spawn(async move { KEY_VALUE_STORE.read_kvp(MEGA_PANEL_KEY) })
            .await
            .map_err(|e| anyhow!("Failed to load mega panel: {}", e))
            .context("loading mega panel")
            .log_err()
            .flatten()
            .map(|panel| serde_json::from_str::<SerializedMegaPanel>(&panel))
            .transpose()
            .log_err()
            .flatten();

        workspace.update(&mut cx, |workspace, cx| {
            let panel = MegaPanel::new(workspace, cx);
            if let Some(serialized_panel) = serialized_panel {
                panel.update(cx, |panel, cx| {
                    panel.width = serialized_panel.width.map(|px| px.round());
                    cx.notify();
                });
            }
            panel
        })
    }

    fn new(workspace: &mut Workspace, cx: &mut ViewContext<Workspace>) -> View<Self> {
        let mega_panel = cx.new_view(|cx| {
            let mega = workspace.mega();
            let focus_handle = cx.focus_handle();
            cx.on_focus(&focus_handle, Self::focus_in).detach();

            #[allow(unused)]
            cx.subscribe(mega, |this, mega, event, cx| {
                // TODO: listen for mega events
            })
            .detach();

            mega.update(cx, |this, cx| {
                this.heartbeat(cx);
            });

            Self {
                mega_handle: mega.clone(),
                // workspace: workspace.weak_handle(),
                focus_handle,
                // scroll_handle: UniformListScrollHandle::new(),
                fs: workspace.app_state().fs.clone(),
                pending_serialization: Task::ready(None),
                width: None,
            }
        });

        mega_panel
    }

    fn serialize(&mut self, cx: &mut ViewContext<Self>) {
        let width = self.width;
        self.pending_serialization = cx.background_executor().spawn(
            async move {
                KEY_VALUE_STORE
                    .write_kvp(
                        MEGA_PANEL_KEY.into(),
                        serde_json::to_string(&SerializedMegaPanel { width })?,
                    )
                    .await?;
                anyhow::Ok(())
            }
            .log_err(),
        );
    }

    fn focus_in(&mut self, cx: &mut ViewContext<Self>) {
        if !self.focus_handle.contains_focused(cx) {
            cx.emit(Event::Focus);
        }
    }

    pub fn checkout_path(&mut self, _: &CheckoutPath, cx: &mut ViewContext<Self>) {
        self.warn_unimplemented(cx);
    }

    pub fn toggle_fuse_mount(&mut self, _: &ToggleFuseMount, cx: &mut ViewContext<Self>) {
        self.mega_handle.update(cx, |this, cx| {
            this.toggle_mount(cx);
        });
    }

    fn render_status(&mut self, cx: &mut ViewContext<Self>) -> Div {
        let (fuse_running, fuse_mounted) = self.mega_handle.read(cx).status();

        v_flex().gap_1().children([
            self.status_unit(cx, "Scorpio Backend:", fuse_running),
            self.status_unit(cx, "Fuse Mounted:", fuse_mounted),
        ])
    }

    fn render_checkout_points(&mut self, cx: &mut ViewContext<Self>) -> Stateful<Div> {
        let points = self.mega_handle.read(cx).checkout_points();
        let points: Vec<Label> = points.into_iter().map(|t| {
            Label::new(t.to_owned()).single_line().color(Color::Ignored)
        }).collect();
        
        v_flex().id("checkout_points").gap_1().text_ui(cx)
            .child(Label::new("Checkout Points:").single_line())
            .children(points)
            
    }

    fn render_buttons(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        fn encap_btn(btn: Button) -> Div {
            div().m_1().border_1().child(btn)
        }

        v_flex().id("mega-control-pad").size_full().children([
            encap_btn(
                Button::new("refresh_mega_status", "Refresh Status")
                    .full_width()
                    .icon(IconName::Control)
                    .icon_position(IconPosition::Start)
                    .on_click(cx.listener(|this, _, cx | {
                        this.mega_handle.update(cx, |mega, cx| { 
                            mega.update_status(cx);
                            println!("{mega:?}");
                        })
                    }))
            ),
            encap_btn(
                Button::new("btn_toggle_mount", "Toggle Fuse Running")
                    .full_width()
                    .icon(IconName::Context)
                    .icon_position(IconPosition::Start)
                    .on_click(cx.listener(|this, _, cx| {
                        this.mega_handle.update(cx, |mega, cx| mega.toggle_mount(cx));
                    })),
            ),
            encap_btn(
                Button::new("btn_toggle_scorpio", "Toggle Scorpio Checkouts")
                    .full_width()
                    .icon(IconName::Plus)
                    .icon_position(IconPosition::Start)
                    .on_click(cx.listener(|this, _, cx| {
                        this.mega_handle.update(cx, |mega, cx| mega.toggle_fuse(cx));
                    })),
            ),
        ])
    }

    fn status_unit(
        &self,
        cx: &mut ViewContext<MegaPanel>,
        name: &'static str,
        state: bool,
    ) -> Stateful<Div> {
        let unit_id = ElementId::from(SharedString::from(format!("status_{}", name)));
        div().text_ui(cx).id(unit_id).child(
            h_flex()
                .justify_between()
                .child(Label::new(name))
                .child(match state {
                    true => Label::new("Active").color(Color::Success),
                    false => Label::new("Inactive").color(Color::Error),
                }),
        )
    }

    fn warn_unimplemented(&self, cx: &mut ViewContext<Self>) {
        let message = String::from(
            "This operation is not implemented yet, functions may not behave correctly",
        );
        let _ = cx.prompt(
            PromptLevel::Warning,
            "Unimplemented",
            Some(&message),
            &["Got it"],
        );
    }
}

fn horizontal_separator(cx: &mut WindowContext) -> Div {
    div().mx_2().border_primary(cx).border_t_1()
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use workspace::AppState;

    #[gpui::test]
    async fn test_mega_panel_functions(cx: &mut TestAppContext) {
        let state = cx.update(|cx| {
            let state = AppState::test(cx);
            mega::init(cx);
            state
        });

        state.mega.update(cx, |this, cx| {
            let recv = this.get_fuse_config(cx);
            cx.spawn(|_this, _cx| async move {
                match recv.await.unwrap() {
                    None => panic!("Failed to get config"),
                    Some(config) => {
                        eprintln!("{config:?}");
                    }
                }
            })
            .detach();
        });
    }
}
