use cosmic::iced::widget::tooltip::Position;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, row, scrollable, text, text_input, toggler, tooltip};
use cosmic::Element;

use crate::app::{Message, PendingAction, SystemdApplet};
use crate::systemd::UnitInfo;

/// Truncates a string to at most `max_chars` characters, appending `…` if truncated,
/// so single-line labels never wrap or overlap with adjacent UI elements.
fn truncate_text(s: &str, max_chars: usize) -> String {
    let first_line = s.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= max_chars {
        first_line.to_string()
    } else {
        let truncated: String = first_line
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect();
        format!("{}…", truncated.trim_end())
    }
}

pub fn view_main(state: &SystemdApplet) -> Element<'_, Message> {
    let mut list = column![].spacing(12);
    if state.services.is_empty() && state.error_message.is_none() {
        list = list.push(text("Loading services..."));
    } else if state.pinned_services.is_empty() {
        list = list.push(text("No pinned services found."));
    } else {
        for pinned in &state.pinned_services {
            let (is_active, status_text) =
                if let Some(service) = state.services.iter().find(|s| s.id == pinned.id) {
                    (
                        state.is_service_active(service),
                        state.service_status_text(service),
                    )
                } else {
                    let pending_active = matches!(
                        state.pending_actions.get(&pinned.id),
                        Some(PendingAction::Starting | PendingAction::Restarting)
                    );
                    (pending_active, "inactive (dead)".to_string())
                };

            let unit_id = pinned.id.clone();
            let unit_id_for_toggle = unit_id.clone();

            let display_name = if pinned.label.is_empty() {
                &pinned.id
            } else {
                &pinned.label
            };

            let info_col = column![
                text(truncate_text(display_name, 30)).size(15),
                text(truncate_text(&status_text, 36)).size(12),
            ]
            .spacing(3)
            .width(Length::Fill);

            let row_content = row![
                info_col,
                toggler(is_active).on_toggle(move |t| {
                    if t {
                        Message::Start(unit_id_for_toggle.clone())
                    } else {
                        Message::Stop(unit_id_for_toggle.clone())
                    }
                }),
                button::icon(cosmic::widget::icon::from_name("view-refresh-symbolic"))
                    .on_press(Message::Restart(unit_id)),
            ]
            .spacing(10)
            .align_y(Alignment::Center);

            list = list.push(row_content);
        }
    }

    let manage_btn = button::standard("Manage Services...")
        .on_press(Message::GoToManage)
        .width(Length::Fill);

    column![text("Services").size(18), list, manage_btn]
        .spacing(12)
        .width(Length::Fixed(380.0))
        .into()
}

pub fn view_manage(state: &SystemdApplet) -> Element<'_, Message> {
    let header = row![
        text("Manage Services").size(18).width(Length::Fill),
        button::standard("Done").on_press(Message::GoToMain)
    ]
    .align_y(Alignment::Center);

    let mut list = column![].spacing(10);
    for (i, pinned) in state.pinned_services.iter().enumerate() {
        let label_text = if pinned.label.is_empty() {
            &pinned.id
        } else {
            &pinned.label
        };

        let text_col = column![
            text(truncate_text(label_text, 26)).size(15),
            text(truncate_text(&pinned.id, 32)).size(12),
        ]
        .spacing(3)
        .width(Length::Fill);

        let btn_up = tooltip(
            button::icon(cosmic::widget::icon::from_name("go-up-symbolic"))
                .on_press(Message::MoveUp(i)),
            text("Move Up"),
            Position::Top,
        );

        let btn_down = tooltip(
            button::icon(cosmic::widget::icon::from_name("go-down-symbolic"))
                .on_press(Message::MoveDown(i)),
            text("Move Down"),
            Position::Top,
        );

        let btn_edit = tooltip(
            button::icon(cosmic::widget::icon::from_name("document-edit-symbolic"))
                .on_press(Message::GoToEdit(i)),
            text("Edit Label"),
            Position::Top,
        );

        let btn_delete = tooltip(
            button::icon(cosmic::widget::icon::from_name("edit-delete-symbolic"))
                .class(cosmic::theme::Button::Destructive)
                .on_press(Message::RemoveService(i)),
            text("Remove Service"),
            Position::Top,
        );

        let row_content = row![text_col, btn_up, btn_down, btn_edit, btn_delete,]
            .spacing(5)
            .align_y(Alignment::Center);

        list = list.push(row_content);
    }

    let add_row = row![
        text_input("Add systemd unit...", &state.add_service_input)
            .on_input(Message::AddServiceInputChanged)
            .on_submit(|_| Message::AddService)
            .width(Length::Fill),
        button::standard("Add").on_press(Message::AddService)
    ]
    .spacing(10);

    let mut manage_content = column![
        header,
        scrollable(list).height(Length::Fixed(300.0)),
        add_row
    ]
    .spacing(10);

    if let Some(ref err) = state.add_service_error {
        manage_content = manage_content.push(text(err).class(cosmic::theme::Text::Color(
            cosmic::iced::Color::from_rgb(0.9, 0.2, 0.2),
        )));
    }

    let query = state.add_service_input.trim().to_lowercase();
    if !query.is_empty() {
        let mut matches: Vec<(u8, &UnitInfo)> = state
            .services
            .iter()
            .filter(|service| {
                !state
                    .pinned_services
                    .iter()
                    .any(|pinned| pinned.id == service.id)
            })
            .filter_map(|service| {
                let id_lower = service.id.to_lowercase();
                let desc_lower = service.description.to_lowercase();
                if id_lower.starts_with(&query) {
                    Some((0, service))
                } else if id_lower.contains(&query) {
                    Some((1, service))
                } else if desc_lower.contains(&query) {
                    Some((2, service))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|(rank_a, a), (rank_b, b)| rank_a.cmp(rank_b).then_with(|| a.id.cmp(&b.id)));

        let mut autocomplete_list = column![].spacing(6);
        for (_, service) in matches.into_iter().take(25) {
            let btn = button::text(truncate_text(&service.id, 38))
                .on_press(Message::GoToConfirmAdd(service.id.clone()))
                .width(Length::Fill);
            autocomplete_list = autocomplete_list.push(btn);
        }
        let autocomplete_scroll = scrollable(autocomplete_list).height(Length::Fixed(180.0));
        manage_content = manage_content.push(autocomplete_scroll);
    }

    manage_content.width(Length::Fixed(400.0)).into()
}

pub fn view_edit(state: &SystemdApplet, idx: usize) -> Element<'_, Message> {
    let header = text("Edit Custom Label").size(18);

    let service_id = if let Some(s) = state.pinned_services.get(idx) {
        &s.id
    } else {
        ""
    };
    let subtitle = text(format!("Editing unit: {}", truncate_text(service_id, 34))).size(14);

    let edit_row = text_input("Custom Label", &state.edit_label_input)
        .id(cosmic::iced::widget::Id::new("edit_label_input"))
        .on_input(Message::EditLabelInputChanged)
        .on_submit(move |_| Message::SaveEditLabel(idx));

    let actions = row![
        button::standard("Cancel").on_press(Message::GoToManage),
        button::standard("Save").on_press(Message::SaveEditLabel(idx)),
    ]
    .spacing(10);

    column![header, subtitle, edit_row, actions]
        .spacing(10)
        .width(Length::Fixed(400.0))
        .into()
}

pub fn view_confirm_add<'a>(state: &'a SystemdApplet, id: &'a str) -> Element<'a, Message> {
    let header = text(format!("Add Service: {}", truncate_text(id, 28))).size(18);

    let id_clone = id.to_string();
    let edit_row = text_input("Custom Label", &state.edit_label_input)
        .id(cosmic::iced::widget::Id::new("confirm_label_input"))
        .on_input(Message::EditLabelInputChanged)
        .on_submit(move |_| Message::SaveNewService(id_clone.clone()));

    let id_clone2 = id.to_string();
    let actions = row![
        button::standard("Cancel").on_press(Message::GoToManage),
        button::standard("Save/Pin").on_press(Message::SaveNewService(id_clone2)),
    ]
    .spacing(10);

    column![header, edit_row, actions]
        .spacing(10)
        .width(Length::Fixed(400.0))
        .into()
}
