use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, responsive};
use iced::{Alignment, Element, Length, Color};
use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;
use crate::ui::page_header;

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
    
    let theme_cloned = theme.clone();
    
    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < 750.0;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        
        // Add subscription input form
        let input = text_input(tr(lang, "sub_url_placeholder"), url_input)
            .on_input(Message::SubscriptionInputChanged)
            .on_submit(Message::DownloadSubscription)
            .padding(12)
            .width(Length::Fill)
            .style(theme::input_field);
            
        let download_btn = if downloading {
            button(
                text(tr(lang, "btn_downloading"))
                    .size(14)
                    .align_x(Alignment::Center)
            )
            .padding([12, 24])
            .style(theme::button_secondary)
        } else {
            button(
                text(tr(lang, "btn_download"))
                    .size(14)
                    .align_x(Alignment::Center)
            )
            .padding([12, 24])
            .style(theme::button_primary)
            .on_press(Message::DownloadSubscription)
        };
        
        let open_folder_btn = button(
            text(tr(lang, "btn_open_folder"))
                .size(14)
                .align_x(Alignment::Center)
        )
        .padding([12, 24])
        .style(theme::button_secondary)
        .on_press(Message::OpenProfilesFolder);
        
        let form_layout: Element<'a, Message> = if is_compact {
            column![
                input,
                row![
                    download_btn.width(Length::Fill),
                    open_folder_btn.width(Length::Fill)
                ]
                .spacing(15)
                .width(Length::Fill)
            ]
            .spacing(12)
            .width(Length::Fill)
            .into()
        } else {
            row![
                input,
                download_btn,
                open_folder_btn
            ]
            .spacing(15)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into()
        };
        
        let add_form = container(
            column![
                text(tr(lang, "import_sub")).color(text_muted).size(14),
                form_layout
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
        
        // Grid system for profiles list (Responsive Grid)
        let grid_content: Element<'a, Message> = if gui_config.subscriptions.is_empty() {
            container(
                text(tr(lang, "no_profiles"))
                    .color(text_muted)
                    .size(14)
            )
            .padding(25)
            .width(Length::Fill)
            .style(theme::card_bg)
            .into()
        } else {
            let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
            
            for profile in &gui_config.subscriptions {
                let is_active = Some(&profile.id) == gui_config.active_profile_id.as_ref();
                
                let update_btn = button(text(tr(lang, "btn_update")).size(11))
                    .padding([5, 10])
                    .style(theme::button_primary)
                    .on_press(Message::UpdateSubscription(profile.id.clone()));
          
                let delete_btn = if confirm_delete_id == Some(profile.id.as_str()) {
                    button(text(tr(lang, "confirm_delete_profile")).size(11))
                        .padding([5, 10])
                        .style(theme::button_danger)
                        .on_press(Message::DeleteProfile(profile.id.clone()))
                } else {
                    button(text(tr(lang, "btn_delete")).size(11))
                        .padding([5, 10])
                        .style(theme::button_secondary)
                        .on_press(Message::DeleteProfile(format!("confirm:{}", profile.id)))
                };
                    
                let edit_btn = button(text(tr(lang, "btn_edit")).size(11))
                    .padding([5, 10])
                    .style(theme::button_secondary)
                    .on_press(Message::EditProfile(profile.id.clone()));
                    
                let badge_or_spacer: Element<'a, Message> = if is_active {
                    container(text(tr(lang, "active_profile")).color(Color::WHITE).size(11))
                        .padding([5, 10])
                        .style(|_theme: &iced::Theme| container::Style {
                            background: Some(iced::Background::Color(theme::SUCCESS)),
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .into()
                } else {
                    iced::widget::Space::new().width(Length::Shrink).into()
                };
                
                let mut actions_row = row![].spacing(8).align_y(Alignment::Center);
                if !is_active {
                    let select_btn = button(text(tr(lang, "btn_select")).size(11))
                        .padding([5, 10])
                        .style(theme::button_secondary)
                        .on_press(Message::SelectProfile(profile.id.clone()));
                    actions_row = actions_row.push(select_btn);
                }
                actions_row = actions_row
                    .push(update_btn)
                    .push(edit_btn)
                    .push(delete_btn);
                    
                let display_url = if profile.url.chars().count() > 40 {
                    let truncated: String = profile.url.chars().take(40).collect();
                    format!("{}...", truncated)
                } else {
                    profile.url.clone()
                };

                let card_layout = column![
                    column![
                        text(&profile.name)
                            .color(text_primary)
                            .size(15)
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
                
                let profile_card = container(card_layout)
                    .padding(20)
                    .width(Length::Fill)
                    .style(move |theme| {
                        if is_active {
                            theme::card_selected(theme)
                        } else {
                            theme::card_bg(theme)
                        }
                    });
                
                card_elements.push(container(profile_card).width(Length::Fill).into());
            }

            // Calculate columns based on width
            let cols = if size.width < 650.0 {
                1
            } else if size.width < 950.0 {
                2
            } else {
                3
            };

            let mut grid_rows = Column::new().spacing(15);
            let mut current_row = iced::widget::Row::new().spacing(15);
            let total_cards = card_elements.len();

            for (i, card) in card_elements.into_iter().enumerate() {
                current_row = current_row.push(container(card).width(Length::FillPortion(1)));
                if (i + 1) % cols == 0 {
                    grid_rows = grid_rows.push(current_row);
                    current_row = iced::widget::Row::new().spacing(15);
                }
            }
            
            let remaining_elements = total_cards % cols;
            if remaining_elements > 0 {
                for _ in remaining_elements..cols {
                    current_row = current_row.push(container(text("")).width(Length::FillPortion(1)));
                }
                grid_rows = grid_rows.push(current_row);
            }
            scrollable(grid_rows).height(Length::Fill).into()
        };
        
        let mut main_layout_col = column![
            add_form,
        ]
        .spacing(20)
        .width(Length::Fill);
        
        if let Some(banner) = error_banner {
            main_layout_col = main_layout_col.push(banner);
        }
        
        main_layout_col = main_layout_col.push(text(tr(lang, "imported_profiles")).color(text_muted).size(14));
        main_layout_col = main_layout_col.push(grid_content);
        
        let header = page_header("tab_profiles", lang, None, theme, is_compact);
        
        let col = column![header, main_layout_col].spacing(20).width(Length::Fill).height(Length::Fill);

        container(col)
            .width(Length::Fill)
            .max_width(1200.0)
            .center_x(Length::Fill)
            .padding(crate::ui::page_padding())
            .into()
    });
    
    main_content.into()
}
