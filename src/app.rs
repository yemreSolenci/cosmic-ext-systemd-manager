use cosmic::app::{Application, Core, Task as AppTask};
use cosmic::iced::core::window::Id as WindowId;
use cosmic::iced::Subscription;
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::Element;
use std::collections::HashMap;
use std::time::Duration;

use crate::config::{load_pinned_services, save_pinned_services, PinnedService};
use crate::systemd::{SystemdClient, UnitInfo};
use crate::ui::{view_confirm_add, view_edit, view_main, view_manage};

#[derive(Debug, Clone, PartialEq)]
pub enum AppletView {
    Main,
    Manage,
    Edit(usize),
    ConfirmAdd(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    Starting,
    Stopping,
    Restarting,
}

pub struct SystemdApplet {
    pub core: Core,
    pub client: Option<SystemdClient>,
    pub services: Vec<UnitInfo>,
    pub pending_actions: HashMap<String, PendingAction>,
    pub error_message: Option<String>,
    pub pinned_services: Vec<PinnedService>,
    pub current_view: AppletView,
    pub add_service_input: String,
    pub add_service_error: Option<String>,
    pub edit_label_input: String,
    pub popup: Option<WindowId>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ClientReady(Result<SystemdClient, String>),
    ServicesFetched(Result<Vec<UnitInfo>, String>),
    Refresh,
    RefreshFull,
    Start(String),
    Stop(String),
    Restart(String),
    ActionCompleted(String, Result<(), String>),
    PopupClosed(WindowId),
    ClosePopupAndReset(WindowId),
    OpenPopup(cosmic::surface::Action),
    Surface(cosmic::surface::Action),
    GoToMain,
    GoToManage,
    GoToEdit(usize),
    MoveUp(usize),
    MoveDown(usize),
    RemoveService(usize),
    AddServiceInputChanged(String),
    AddService,
    GoToConfirmAdd(String),
    SaveNewService(String),
    EditLabelInputChanged(String),
    SaveEditLabel(usize),
}

impl SystemdApplet {
    /// Determines whether the toggle button for a service should be shown as active (ON).
    pub fn is_service_active(&self, service: &UnitInfo) -> bool {
        if let Some(pending) = self.pending_actions.get(&service.id) {
            return match pending {
                PendingAction::Starting | PendingAction::Restarting => true,
                PendingAction::Stopping => false,
            };
        }

        match service.active_state.as_str() {
            "active" | "reloading" => true,
            "activating" => {
                // Actively starting (start-pre, start, start-post) is treated as active;
                // waiting on an auto-restart timer is not yet active and will be polled
                // until the auto-restart completes and the final state is known.
                service.sub_state != "auto-restart"
            }
            _ => false,
        }
    }

    /// Formats the status string shown next to a pinned service.
    pub fn service_status_text(&self, service: &UnitInfo) -> String {
        if let Some(pending) = self.pending_actions.get(&service.id) {
            match pending {
                PendingAction::Starting if service.active_state != "active" => {
                    return "activating (start)".to_string();
                }
                PendingAction::Stopping if service.active_state != "inactive" => {
                    return "deactivating (stop)".to_string();
                }
                PendingAction::Restarting => {
                    return "restarting...".to_string();
                }
                _ => {}
            }
        }
        format!("{} ({})", service.active_state, service.sub_state)
    }

    /// Returns true if any pinned service (or pending action) is in a transitional state
    /// such as `activating`, `deactivating`, `reloading`, or `auto-restart`.
    pub fn has_transitioning_services(&self) -> bool {
        if !self.pending_actions.is_empty() {
            return true;
        }

        self.pinned_services.iter().any(|pinned| {
            self.services
                .iter()
                .find(|s| s.id == pinned.id)
                .map(|s| {
                    s.job_id != 0
                        || s.active_state == "activating"
                        || s.active_state == "deactivating"
                        || s.active_state == "reloading"
                        || s.sub_state == "auto-restart"
                })
                .unwrap_or(false)
        })
    }
}

impl Application for SystemdApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Message = Message;
    type Flags = ();

    const APP_ID: &'static str = "io.github.yemresolenci.CosmicExtAppletSystemdManager";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, AppTask<Self::Message>) {
        (
            SystemdApplet {
                core,
                client: None,
                services: Vec::new(),
                pending_actions: HashMap::new(),
                error_message: None,
                pinned_services: load_pinned_services(),
                current_view: AppletView::Main,
                add_service_input: String::new(),
                add_service_error: None,
                edit_label_input: String::new(),
                popup: None,
            },
            AppTask::perform(
                async { SystemdClient::new().await.map_err(|e| e.to_string()) },
                |res| cosmic::Action::App(Message::ClientReady(res)),
            ),
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.client.is_none() {
            return Subscription::none();
        }

        let interval = if self.has_transitioning_services() || self.popup.is_some() {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(3)
        };

        cosmic::iced::time::every(interval).map(|_| Message::Refresh)
    }

    fn on_close_requested(&self, id: WindowId) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> AppTask<Self::Message> {
        match message {
            Message::ClientReady(Ok(client)) => {
                let client_clone = client.clone();
                self.client = Some(client);
                return AppTask::perform(
                    async move {
                        client_clone
                            .list_services_full()
                            .await
                            .map_err(|e| e.to_string())
                    },
                    |res| cosmic::Action::App(Message::ServicesFetched(res)),
                );
            }
            Message::ClientReady(Err(e)) => {
                self.error_message = Some(e);
            }
            Message::ServicesFetched(Ok(services)) => {
                self.services = services;
                self.pending_actions.retain(|id, action| {
                    if let Some(svc) = self.services.iter().find(|s| &s.id == id) {
                        match action {
                            PendingAction::Starting | PendingAction::Restarting => {
                                svc.job_id != 0
                                    || (svc.active_state == "activating"
                                        && svc.sub_state != "auto-restart")
                                    || svc.active_state == "reloading"
                            }
                            PendingAction::Stopping => {
                                svc.job_id != 0
                                    || svc.active_state == "deactivating"
                                    || svc.active_state == "active"
                            }
                        }
                    } else {
                        false
                    }
                });
            }
            Message::ServicesFetched(Err(e)) => {
                self.error_message = Some(e);
            }
            Message::Refresh => {
                if let Some(client) = &self.client {
                    let client_clone = client.clone();
                    return AppTask::perform(
                        async move {
                            client_clone
                                .list_services()
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |res| cosmic::Action::App(Message::ServicesFetched(res)),
                    );
                }
            }
            Message::RefreshFull => {
                if let Some(client) = &self.client {
                    let client_clone = client.clone();
                    return AppTask::perform(
                        async move {
                            client_clone
                                .list_services_full()
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |res| cosmic::Action::App(Message::ServicesFetched(res)),
                    );
                }
            }
            Message::Start(unit_id) => {
                if let Some(client) = &self.client {
                    self.pending_actions
                        .insert(unit_id.clone(), PendingAction::Starting);
                    let client_clone = client.clone();
                    let id_for_done = unit_id.clone();
                    return AppTask::perform(
                        async move {
                            client_clone
                                .start_unit(&unit_id)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |res| {
                            cosmic::Action::App(Message::ActionCompleted(id_for_done.clone(), res))
                        },
                    );
                }
            }
            Message::Stop(unit_id) => {
                if let Some(client) = &self.client {
                    self.pending_actions
                        .insert(unit_id.clone(), PendingAction::Stopping);
                    let client_clone = client.clone();
                    let id_for_done = unit_id.clone();
                    return AppTask::perform(
                        async move {
                            client_clone
                                .stop_unit(&unit_id)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |res| {
                            cosmic::Action::App(Message::ActionCompleted(id_for_done.clone(), res))
                        },
                    );
                }
            }
            Message::Restart(unit_id) => {
                if let Some(client) = &self.client {
                    self.pending_actions
                        .insert(unit_id.clone(), PendingAction::Restarting);
                    let client_clone = client.clone();
                    let id_for_done = unit_id.clone();
                    return AppTask::perform(
                        async move {
                            client_clone
                                .restart_unit(&unit_id)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |res| {
                            cosmic::Action::App(Message::ActionCompleted(id_for_done.clone(), res))
                        },
                    );
                }
            }
            Message::ActionCompleted(unit_id, res) => {
                self.pending_actions.remove(&unit_id);
                match res {
                    Ok(()) => self.error_message = None,
                    Err(e) => self.error_message = Some(e),
                }
                let immediate_refresh =
                    AppTask::perform(async {}, |_| cosmic::Action::App(Message::Refresh));
                let delayed_refresh = AppTask::perform(
                    async {
                        tokio::time::sleep(Duration::from_millis(1200)).await;
                    },
                    |_| cosmic::Action::App(Message::Refresh),
                );
                return AppTask::batch(vec![immediate_refresh, delayed_refresh]);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.current_view = AppletView::Main;
                }
            }
            Message::ClosePopupAndReset(id) => {
                self.popup = None;
                self.current_view = AppletView::Main;
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(destroy_popup(id)),
                ));
            }
            Message::OpenPopup(a) => {
                let open_task = cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(a),
                ));
                let refresh_task =
                    AppTask::perform(async {}, |_| cosmic::Action::App(Message::RefreshFull));
                return AppTask::batch(vec![open_task, refresh_task]);
            }
            Message::Surface(a) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(a),
                ));
            }
            Message::GoToMain => {
                self.current_view = AppletView::Main;
                return AppTask::perform(async {}, |_| cosmic::Action::App(Message::Refresh));
            }
            Message::GoToManage => {
                self.current_view = AppletView::Manage;
                self.add_service_input.clear();
                self.add_service_error = None;
                return AppTask::perform(async {}, |_| cosmic::Action::App(Message::RefreshFull));
            }
            Message::GoToEdit(idx) => {
                if let Some(service) = self.pinned_services.get(idx) {
                    self.edit_label_input = if service.label.is_empty() {
                        service.id.clone()
                    } else {
                        service.label.clone()
                    };
                    self.current_view = AppletView::Edit(idx);

                    let focus_task = cosmic::widget::text_input::focus::<Message>(
                        cosmic::iced::widget::Id::new("edit_label_input"),
                    );
                    return focus_task.map(cosmic::Action::App);
                }
            }
            Message::MoveUp(idx) => {
                if idx > 0 && idx < self.pinned_services.len() {
                    self.pinned_services.swap(idx, idx - 1);
                    save_pinned_services(&self.pinned_services);
                }
            }
            Message::MoveDown(idx) => {
                if idx + 1 < self.pinned_services.len() {
                    self.pinned_services.swap(idx, idx + 1);
                    save_pinned_services(&self.pinned_services);
                }
            }
            Message::RemoveService(idx) => {
                if idx < self.pinned_services.len() {
                    self.pinned_services.remove(idx);
                    save_pinned_services(&self.pinned_services);
                }
            }
            Message::AddServiceInputChanged(val) => {
                self.add_service_input = val;
                self.add_service_error = None;
            }
            Message::AddService => {
                let raw = self.add_service_input.trim();
                if !raw.is_empty() {
                    let candidate_with_ext = if raw.ends_with(".service") {
                        raw.to_string()
                    } else {
                        format!("{}.service", raw)
                    };

                    let matched_id = self
                        .services
                        .iter()
                        .find(|u| {
                            u.id.eq_ignore_ascii_case(raw)
                                || u.id.eq_ignore_ascii_case(&candidate_with_ext)
                        })
                        .map(|u| u.id.clone());

                    if let Some(s) = matched_id {
                        if self.pinned_services.iter().any(|pinned| pinned.id == s) {
                            self.add_service_error = Some("Service is already added.".to_string());
                        } else {
                            self.edit_label_input = s.clone();
                            self.current_view = AppletView::ConfirmAdd(s);
                            self.add_service_input.clear();
                            self.add_service_error = None;

                            let focus_task = cosmic::widget::text_input::focus::<Message>(
                                cosmic::iced::widget::Id::new("confirm_label_input"),
                            );
                            return focus_task.map(cosmic::Action::App);
                        }
                    } else {
                        self.add_service_error = Some("Service not found.".to_string());
                    }
                }
            }
            Message::GoToConfirmAdd(s) => {
                let s = s.trim().to_string();
                if !s.is_empty() {
                    if self.pinned_services.iter().any(|pinned| pinned.id == s) {
                        self.add_service_error = Some("Service is already added.".to_string());
                    } else {
                        self.edit_label_input = s.clone();
                        self.current_view = AppletView::ConfirmAdd(s);
                        self.add_service_input.clear();
                        self.add_service_error = None;

                        let focus_task = cosmic::widget::text_input::focus::<Message>(
                            cosmic::iced::widget::Id::new("confirm_label_input"),
                        );
                        return focus_task.map(cosmic::Action::App);
                    }
                }
            }
            Message::SaveNewService(id) => {
                let label = if self.edit_label_input.trim().is_empty() {
                    id.clone()
                } else {
                    self.edit_label_input.trim().to_string()
                };
                self.pinned_services.push(PinnedService { id, label });
                save_pinned_services(&self.pinned_services);
                self.current_view = AppletView::Manage;
            }
            Message::EditLabelInputChanged(val) => {
                self.edit_label_input = val;
            }
            Message::SaveEditLabel(idx) => {
                if let Some(service) = self.pinned_services.get_mut(idx) {
                    service.label = self.edit_label_input.trim().to_string();
                    save_pinned_services(&self.pinned_services);
                }
                self.current_view = AppletView::Manage;
            }
        }
        AppTask::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let have_popup = self.popup;
        let btn = self
            .core
            .applet
            .icon_button("preferences-system-symbolic")
            .on_press_with_rectangle(move |offset, bounds| {
                if let Some(id) = have_popup {
                    Message::ClosePopupAndReset(id)
                } else {
                    Message::OpenPopup(app_popup::<SystemdApplet>(
                        move |state: &mut SystemdApplet| {
                            let new_id = WindowId::unique();
                            state.popup = Some(new_id);
                            let main_id = state.core.main_window_id().unwrap_or(new_id);
                            let mut popup_settings = state
                                .core
                                .applet
                                .get_popup_settings(main_id, new_id, None, None, None);

                            popup_settings.positioner.anchor_rect = cosmic::iced::Rectangle {
                                x: (bounds.x - offset.x) as i32,
                                y: (bounds.y - offset.y) as i32,
                                width: bounds.width as i32,
                                height: bounds.height as i32,
                            };

                            popup_settings
                        },
                        Some(Box::new(move |state: &SystemdApplet| {
                            let mut content_col = cosmic::widget::column![].spacing(10);
                            if let Some(ref err) = state.error_message {
                                content_col = content_col
                                    .push(cosmic::widget::text(format!("Error: {}", err)));
                            }

                            let view_content = match state.current_view {
                                AppletView::Main => view_main(state),
                                AppletView::Manage => view_manage(state),
                                AppletView::Edit(idx) => view_edit(state, idx),
                                AppletView::ConfirmAdd(ref id) => view_confirm_add(state, id),
                            };

                            let content = content_col.push(view_content).spacing(20).padding(20);

                            Element::from(state.core.applet.popup_container(content))
                                .map(cosmic::Action::App)
                        })),
                    ))
                }
            });

        Element::from(self.core.applet.applet_tooltip::<Message>(
            btn,
            "Systemd Services",
            self.popup.is_some(),
            Message::Surface,
            None,
        ))
    }

    fn view_window(&self, _id: WindowId) -> Element<'_, Message> {
        cosmic::widget::text("Systemd Applet Window").into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
