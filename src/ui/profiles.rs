use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    url_input: &'a str,
    downloading: bool,
) -> Element<'a, Message> {
    
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    // Title
    let title = text(tr(lang, "tab_profiles")).size(24).color(theme::TEXT_PRIMARY);
    
    // Add subscription input form
    let input = text_input(tr(lang, "sub_url_placeholder"), url_input)
        .on_input(Message::SubscriptionInputChanged)
        .padding(12)
        .style(theme::input_field);
        
    let download_btn = if downloading {
        button(text(tr(lang, "btn_downloading")).size(14))
            .padding([12, 24])
            .style(theme::button_secondary)
    } else {
        button(text(tr(lang, "btn_download")).size(14))
            .padding([12, 24])
            .style(theme::button_primary)
            .on_press(Message::DownloadSubscription)
    };
    
    let add_form = container(
        column![
            text(tr(lang, "import_sub")).color(theme::TEXT_MUTED).size(14),
            row![
                input,
                download_btn
            ]
            .spacing(15)
            .align_y(Alignment::Center)
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // List existing profiles
    let mut profiles_col = Column::new().spacing(15);
    
    if gui_config.subscriptions.is_empty() {
        profiles_col = profiles_col.push(
            container(
                text(tr(lang, "no_profiles"))
                    .color(theme::TEXT_MUTED)
                    .size(14)
            )
            .padding(25)
            .width(Length::Fill)
            .style(theme::card_bg)
        );
    } else {
        for profile in &gui_config.subscriptions {
            let is_active = Some(&profile.id) == gui_config.active_profile_id.as_ref();
            
            let status_badge = if is_active {
                container(text(tr(lang, "active_profile")).color(theme::TEXT_PRIMARY).size(12))
                    .padding([4, 8])
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(theme::SUCCESS)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
            } else {
                container(text(tr(lang, "btn_select")).color(theme::TEXT_MUTED).size(12))
                    .padding([4, 8])
                    .style(theme::card_bg)
            };
            
            let select_action = if is_active {
                // If already active, clicking does nothing
                button(status_badge)
                    .style(theme::button_secondary)
            } else {
                button(status_badge)
                    .style(theme::button_secondary)
                    .on_press(Message::SelectProfile(profile.id.clone()))
            };
            
            let delete_btn = button(text(tr(lang, "btn_delete")).size(12).color(theme::TEXT_PRIMARY))
                .padding([6, 12])
                .style(theme::button_danger)
                .on_press(Message::DeleteProfile(profile.id.clone()));
                
            let profile_row = container(
                row![
                    column![
                        text(&profile.name)
                            .color(theme::TEXT_PRIMARY)
                            .size(16)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }),
                        text(&profile.url).color(theme::TEXT_MUTED).size(12),
                        text(format!("{}: {}", tr(lang, "updated_at_label"), profile.updated_at)).color(theme::TEXT_MUTED).size(11),
                    ]
                    .spacing(5)
                    .width(Length::Fill),
                    row![
                        select_action,
                        delete_btn
                    ]
                    .spacing(15)
                    .align_y(Alignment::Center)
                ]
                .align_y(Alignment::Center)
            )
            .padding(15)
            .style(move |theme| {
                if is_active {
                    theme::card_selected(theme)
                } else {
                    theme::card_bg(theme)
                }
            });
            
            profiles_col = profiles_col.push(profile_row);
        }
    }
    
    let scroll_list = scrollable(profiles_col)
        .height(Length::Fill);
        
    container(
        column![
            title,
            add_form,
            text("Imported Profiles").color(theme::TEXT_MUTED).size(14),
            scroll_list
        ]
        .spacing(20)
    )
    .padding(20)
    .into()
}
