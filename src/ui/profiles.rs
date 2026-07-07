use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Alignment, Element, Length, Color};
use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    url_input: &'a str,
    downloading: bool,
    profile_error: Option<&'a str>,
    confirm_delete_id: Option<&'a str>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Title
    let title = text(tr(lang, "tab_profiles")).size(24).color(text_primary);
    
    // Add subscription input form
    let input = text_input(tr(lang, "sub_url_placeholder"), url_input)
        .on_input(Message::SubscriptionInputChanged)
        .on_submit(Message::DownloadSubscription)
        .padding(12)
        .width(Length::Fill)
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
            text(tr(lang, "import_sub")).color(text_muted).size(14),
            row![
                input,
                download_btn,
                button(text(tr(lang, "btn_open_folder")).size(14))
                    .padding([12, 24])
                    .style(theme::button_secondary)
                    .on_press(Message::OpenProfilesFolder)
            ]
            .spacing(15)
            .align_y(Alignment::Center)
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    let error_banner = if let Some(err) = profile_error {
        Some(
            container(
                row![
                    text("⚠️ ").size(14),
                    text(err).size(13).color(theme::DANGER)
                ]
                .align_y(Alignment::Center)
            )
            .padding(12)
            .width(Length::Fill)
            .style(|theme| container::Style {
                background: Some(iced::Background::Color(if theme::is_dark(theme) {
                    iced::Color::from_rgba(0.94, 0.27, 0.27, 0.1)
                } else {
                    iced::Color::from_rgba(0.94, 0.27, 0.27, 0.05)
                })),
                border: iced::Border {
                    color: theme::DANGER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
        )
    } else {
        None
    };
    
    // List existing profiles
    let mut profiles_col = Column::new().spacing(15);
    
    if gui_config.subscriptions.is_empty() {
        profiles_col = profiles_col.push(
            container(
                text(tr(lang, "no_profiles"))
                    .color(text_muted)
                    .size(14)
            )
            .padding(25)
            .width(Length::Fill)
            .style(theme::card_bg)
        );
    } else {
        for profile in &gui_config.subscriptions {
            let is_active = Some(&profile.id) == gui_config.active_profile_id.as_ref();
            
            let update_btn = button(text(tr(lang, "btn_update")).size(12))
                .padding([6, 12])
                .style(theme::button_primary)
                .on_press(Message::UpdateSubscription(profile.id.clone()));
  
            let delete_btn = if confirm_delete_id == Some(&profile.id) {
                button(text(tr(lang, "confirm_delete_profile")).size(12))
                    .padding([6, 12])
                    .style(theme::button_danger)
                    .on_press(Message::DeleteProfile(profile.id.clone()))
            } else {
                button(text(tr(lang, "btn_delete")).size(12))
                    .padding([6, 12])
                    .style(theme::button_secondary)
                    .on_press(Message::DeleteProfile(format!("confirm:{}", profile.id)))
            };
                
            let edit_btn = button(text(tr(lang, "btn_edit")).size(12))
                .padding([6, 12])
                .style(theme::button_secondary)
                .on_press(Message::EditProfile(profile.id.clone()));
                
            let badge_or_spacer: Element<'a, Message> = if is_active {
                container(text(tr(lang, "active_profile")).color(Color::WHITE).size(12))
                    .padding([6, 12])
                    .style(|_theme: &iced::Theme| container::Style {
                        background: Some(iced::Background::Color(theme::SUCCESS)),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                iced::widget::Space::new().width(Length::Shrink).into()
            };
            
            let mut actions_row = row![].spacing(10).align_y(Alignment::Center);
            if !is_active {
                let select_btn = button(text(tr(lang, "btn_select")).size(12))
                    .padding([6, 12])
                    .style(theme::button_secondary)
                    .on_press(Message::SelectProfile(profile.id.clone()));
                actions_row = actions_row.push(select_btn);
            }
            actions_row = actions_row
                .push(update_btn)
                .push(edit_btn)
                .push(delete_btn);
                
            let display_url = if profile.url.chars().count() > 60 {
                let truncated: String = profile.url.chars().take(60).collect();
                format!("{}...", truncated)
            } else {
                profile.url.clone()
            };

            let card_layout = column![
                column![
                    text(&profile.name)
                        .color(text_primary)
                        .size(16)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        }),
                    text(display_url).color(text_muted).size(12),
                    text(format!("{}: {}", tr(lang, "updated_at_label"), profile.updated_at)).color(text_muted).size(11),
                ]
                .spacing(6)
                .width(Length::Fill),
                row![
                    badge_or_spacer,
                    iced::widget::Space::new().width(Length::Fill),
                    actions_row
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(12);
            
            let profile_row = container(card_layout)
                .padding(25)
                .width(Length::Fill)
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
        
    let mut main_layout_col = column![
        title,
        add_form,
    ]
    .spacing(20);
    
    if let Some(banner) = error_banner {
        main_layout_col = main_layout_col.push(banner);
    }
    
    main_layout_col = main_layout_col.push(text(tr(lang, "imported_profiles")).color(text_muted).size(14));
    main_layout_col = main_layout_col.push(scroll_list);
    
    container(
        container(main_layout_col)
            .width(Length::Fill)
            .max_width(800.0)
            .center_x(Length::Fill)
            .padding(30)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
