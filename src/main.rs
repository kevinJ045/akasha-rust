mod utils;
use colored::Color;
use eframe::{egui, App, CreationContext, Frame};
use egui_extras::RetainedImage;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

struct CharacterIcon {
	texture: RetainedImage,
	loading: bool,
	error: bool,
}

struct MyApp {
	characters: Option<Vec<Value>>,
	calculations: Option<Value>,
	selected_character: Option<usize>,
	loading: bool,
	error: Option<String>,
	rx: Receiver<Result<(Vec<Value>, Value), String>>,
	tx: Sender<Result<(Vec<Value>, Value), String>>,
	icons: Arc<Mutex<HashMap<String, CharacterIcon>>>,
	runtime: Arc<tokio::runtime::Runtime>,
}

impl MyApp {
	fn new(cc: &CreationContext) -> Self {
		let (tx, rx) = channel();
		let tx_clone = tx.clone();

		// Create a new runtime for the background thread
		let rt = tokio::runtime::Runtime::new().unwrap();

		std::thread::spawn(move || {
			rt.block_on(async {
				let uid = "772493838";
				match (
					utils::get_user_builds(uid).await,
					utils::get_user_calculations(uid).await,
				) {
					(Ok(chars), Ok(calcs)) => {
						if let Some(char_array) = chars.as_array() {
							tx_clone.send(Ok((char_array.to_vec(), calcs))).unwrap();
						}
					}
					_ => tx_clone
						.send(Err("Failed to fetch data".to_string()))
						.unwrap(),
				}
			});
		});

		// Create another runtime for icon loading
		let rt = tokio::runtime::Runtime::new().unwrap();
		let rt = Arc::new(rt);
		let rt_clone = rt.clone();

		Self {
			characters: None,
			calculations: None,
			selected_character: None,
			loading: true,
			error: None,
			rx,
			tx,
			icons: Arc::new(Mutex::new(HashMap::new())),
			runtime: rt, // Add this field to store the runtime
		}
	}

	async fn load_icon(url: &str) -> Result<RetainedImage, Box<dyn std::error::Error>> {
		let response = reqwest::get(url).await?;
		let bytes = response.bytes().await?;
		let image = RetainedImage::from_image_bytes(url, &bytes)?;
		Ok(image)
	}

	fn ensure_icon(&self, icon_name: &str) {
		let icons = self.icons.clone();
		let icon_url = if icon_name.starts_with("http") {
			icon_name.to_string()
		} else {
			format!("https://enka.network/ui/{}.png", icon_name)
		};

		if !icons.lock().unwrap().contains_key(&icon_url) {
			icons.lock().unwrap().insert(
				icon_url.clone(),
				CharacterIcon {
					texture: RetainedImage::from_color_image(
						"loading",
						egui::ColorImage::new([32, 32], egui::Color32::GRAY),
					),
					loading: true,
					error: false,
				},
			);

			let icons = icons.clone();
			self.runtime.spawn(async move {
				match Self::load_icon(&icon_url).await {
					Ok(image) => {
						let mut icons = icons.lock().unwrap();
						if let Some(icon) = icons.get_mut(&icon_url) {
							icon.texture = image;
							icon.loading = false;
						}
					}
					Err(_) => {
						let mut icons = icons.lock().unwrap();
						if let Some(icon) = icons.get_mut(&icon_url) {
							icon.error = true;
							icon.loading = false;
						}
					}
				}
			});
		}
	}

	fn load_all_icons(&self, value: &Value) {
		if let Some(obj) = value.as_object() {
			for (key, val) in obj {
				if key == "icon" {
					if let Some(icon) = val.as_str() {
						self.ensure_icon(icon);
					}
				} else {
					// Recursively check nested objects and arrays
					match val {
						Value::Object(_) => self.load_all_icons(&val),
						Value::Array(arr) => {
							for item in arr {
								self.load_all_icons(item);
							}
						}
						_ => {}
					}
				}
			}
		} else if let Some(arr) = value.as_array() {
			for item in arr {
				self.load_all_icons(item);
			}
		}
	}

	fn render_character_list(&mut self, ui: &mut egui::Ui) {
		if let Some(chars) = &self.characters {
			// Pre-load all icons from the entire data structure
			for char in chars.iter() {
				self.load_all_icons(char);
			}

			egui::ScrollArea::vertical().show(ui, |ui| {
				for (idx, char) in chars.iter().enumerate() {
					let name = char["name"].as_str().unwrap_or("Unknown");
					let element = char["characterMetadata"]["element"]
						.as_str()
						.unwrap_or("")
						.to_lowercase();

					let icon_url = if let Some(icon) = char["icon"].as_str() {
						if icon.starts_with("http") {
							icon.to_string()
						} else {
							format!("https://enka.network/ui/{}.png", icon)
						}
					} else {
						String::new()
					};

					let element_color = match element.as_str() {
						"hydro" => egui::Color32::from_rgb(0, 144, 255),
						"pyro" => egui::Color32::from_rgb(255, 69, 0),
						"cryo" => egui::Color32::from_rgb(167, 223, 236),
						"electro" => egui::Color32::from_rgb(178, 132, 255),
						"anemo" => egui::Color32::from_rgb(148, 255, 198),
						"geo" => egui::Color32::from_rgb(255, 198, 93),
						"dendro" => egui::Color32::from_rgb(147, 215, 65),
						_ => egui::Color32::WHITE,
					};

					let is_selected = self.selected_character == Some(idx);

					ui.horizontal(|ui| {
						// Get icon from cache
						if !icon_url.is_empty() {
							if let Some(icon) = self.icons.lock().unwrap().get(&icon_url) {
								let size = 32.0;
								icon.texture.show_size(ui, egui::vec2(size, size));
							}
						}

						if ui
							.selectable_label(
								is_selected,
								egui::RichText::new(name).color(element_color),
							)
							.clicked()
						{
							self.selected_character = Some(idx);
						}
					});
				}
			});
		}
	}

	fn render_character_details(&self, ui: &mut egui::Ui) {
		if let Some(idx) = self.selected_character {
			if let Some(chars) = &self.characters {
				if let Some(char) = chars.get(idx) {
					if let Some(calcs) = &self.calculations {
						let char_calcs = calcs
							.as_object()
							.and_then(|obj| obj.get(&char["characterId"].to_string()))
							.and_then(|calc| calc.as_object());

						let full_height = ui.available_height();

						ui.horizontal(|ui| {
							// Left panel for avatar (40% width)
							ui.allocate_ui_with_layout(
								egui::vec2(ui.available_width() * 0.6, ui.available_height()),
								egui::Layout::top_down(egui::Align::Center),
								|ui| {
									if let Some(name) = char.get("name").and_then(|i| i.as_str()) {
										let icon_url = format!("https://enka.network/ui/UI_Gacha_AvatarImg_{}.png", name);
										self.ensure_icon(&icon_url);

										if let Some(icon) = self.icons.lock().unwrap().get(&icon_url) {
											if !icon.loading && !icon.error {
												let available_width = ui.available_width();
												let available_height = ui.available_height();

												egui::Frame::none()
													.inner_margin(0.0)
													.show(ui, |ui| {
														egui::ScrollArea::neither()
															.max_height(available_height)
															.show(ui, |ui| {
																let image_size = icon.texture.size_vec2();
																let aspect_ratio = image_size.y / image_size.x;
																let display_width = available_width;
																let display_height = display_width * aspect_ratio;

																icon.texture.show_size(ui, egui::vec2(display_width, display_height));
															});
													});
											} else if icon.loading {
												ui.spinner();
												ui.label("Loading character art...");
											} else {
												ui.label("Failed to load character art");
											}
										}
									}
								}
							);

							// Right panel for character details (60% width)
							ui.vertical(|ui| {
								egui::Grid::new("character_details_grid")
									.num_columns(2)
									.spacing([20.0, 20.0])
									.min_col_width(200.0)
									.show(ui, |ui| {
										// Card 1: Character Info and Talents
										{
											egui::Frame::none()
												.fill(ui.style().visuals.extreme_bg_color)
												.rounding(10.0)
												.stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.inner_margin(10.0)
												.show(ui, |ui| {
													ui.set_min_width(200.0);
													ui.vertical(|ui| {
														ui.heading("Character Info");
														ui.label(format!(
															"Level {}/{}",
															char["propMap"]["level"]["val"].as_i64().unwrap_or(0),
															char["propMap"]["ascension"]["val"].as_i64().unwrap_or(0) * 10
														));
														ui.label(format!("Constellation: C{}", char["constellation"].as_i64().unwrap_or(0)));

														ui.add_space(10.0);
														ui.heading("Talents");
														ui.label(format!(
															"Normal Attack: {}",
															char["talentsLevelMap"]["normalAttacks"]["level"].as_i64().unwrap_or(0)
														));
														ui.label(format!(
															"Elemental Skill: {}",
															char["talentsLevelMap"]["elementalSkill"]["level"].as_i64().unwrap_or(0)
														));
														ui.label(format!(
															"Elemental Burst: {}",
															char["talentsLevelMap"]["elementalBurst"]["level"].as_i64().unwrap_or(0)
														));
													});
												});
										}

										// Card 2: Weapon Info
										{
											egui::Frame::none()
												.fill(ui.style().visuals.extreme_bg_color)
												.rounding(10.0)
												.inner_margin(10.0)
												.stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.show(ui, |ui| {
													ui.vertical(|ui| {
														ui.heading("Weapon");
														if let Some(weapon) = char["weapon"].as_object() {
															if let Some(icon) = weapon.get("icon").and_then(|i| i.as_str()) {
																let icon_url = if icon.starts_with("http") {
																	icon.to_string()
																} else {
																	format!("https://enka.network/ui/{}.png", icon)
																};
																if let Some(icon) = self.icons.lock().unwrap().get(&icon_url) {
																	let size = 64.0;
																	icon.texture.show_size(ui, egui::vec2(size, size));
																}
															}
															ui.label(format!(
																"{} R{}",
																weapon["name"].as_str().unwrap_or(""),
																weapon["weaponInfo"]["refinementLevel"]["value"].as_i64().unwrap_or(0) + 1
															));
															ui.label(format!(
																"Level {}/{}",
																weapon["weaponInfo"]["level"].as_i64().unwrap_or(0),
																weapon["weaponInfo"]["promoteLevel"].as_i64().unwrap_or(0) * 10
															));
														}
													});
												});
										}

										ui.end_row();

										// Card 3: Stats
										{
											egui::Frame::none()
												.fill(ui.style().visuals.extreme_bg_color)
												.rounding(10.0)
												.inner_margin(10.0)
												.stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.show(ui, |ui| {
													ui.vertical(|ui| {
														ui.heading("Stats");
														ui.label(format!("HP: {}", utils::format_number(char["stats"]["maxHp"]["value"].as_f64().unwrap_or(0.0))));
														ui.label(format!("ATK: {}", utils::format_number(char["stats"]["atk"]["value"].as_f64().unwrap_or(0.0))));
														ui.label(format!("DEF: {}", utils::format_number(char["stats"]["def"]["value"].as_f64().unwrap_or(0.0))));
														ui.label(format!("Crit Rate: {:.1}%", char["stats"]["critRate"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														ui.label(format!("Crit DMG: {:.1}%", char["stats"]["critDamage"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														ui.label(format!("Energy Recharge: {:.1}%", char["stats"]["energyRecharge"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														ui.label(format!("Elemental Mastery: {}", char["stats"]["elementalMastery"]["value"].as_f64().unwrap_or(0.0) as i64));
													});
												});
										}

										// Card 4: Artifacts and Build Quality
										{
											egui::Frame::none()
												.fill(ui.style().visuals.extreme_bg_color)
												.rounding(10.0)
												.inner_margin(10.0)
												.stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.show(ui, |ui| {
													ui.vertical(|ui| {
														// Build Ranking (if available)
														if let Some(calc) = char_calcs {
															if let (Some(rank), Some(total)) = (
																calc.get("ranking").and_then(|v| v.as_i64()),
																calc.get("outOf").and_then(|v| v.as_i64())
															) {
																let percentage = (rank as f64 / total as f64 * 100.0) as i64;
																ui.colored_label(
																	egui::Color32::from_rgb(255, 215, 0),
																	format!("Top {}% ({}/{})", percentage, rank, total)
																);
															}
														}

														ui.heading("Artifact Sets");
														if let Some(sets) = char["artifactSets"].as_object() {
															for (name, details) in sets {
																if let Some(icon) = details.get("icon").and_then(|i| i.as_str()) {
																	ui.horizontal(|ui| {
																		let icon_url = if icon.starts_with("http") {
																			icon.to_string()
																		} else {
																			format!("https://enka.network/ui/{}.png", icon)
																		};
																		if let Some(icon) = self.icons.lock().unwrap().get(&icon_url) {
																			let size = 32.0;
																			icon.texture.show_size(ui, egui::vec2(size, size));
																		}
																		ui.label(format!("{} ({}pc)", name, details["count"].as_i64().unwrap_or(0)));
																	});
																} else {
																	ui.label(format!("{} ({}pc)", name, details["count"].as_i64().unwrap_or(0)));
																}
															}
														}

														ui.add_space(10.0);
														ui.heading("Build Quality");
														ui.label(format!("Crit Value: {:.2}", char["critValue"].as_f64().unwrap_or(0.0)));
													});
												});
										}
									});
							});
						});
					}
				}
			}
		}
	}
}

impl App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
		if self.loading {
			if let Ok(result) = self.rx.try_recv() {
				match result {
					Ok((chars, calcs)) => {
						self.characters = Some(chars);
						self.calculations = Some(calcs);
						self.loading = false;
					}
					Err(e) => {
						self.error = Some(e);
						self.loading = false;
					}
				}
			}
		}

		egui::SidePanel::left("character_list")
			.default_width(200.0)
			.show(ctx, |ui| {
				ui.heading("Characters");
				if self.loading {
					ui.spinner();
				} else if let Some(error) = &self.error {
					ui.colored_label(egui::Color32::RED, error);
				} else {
					self.render_character_list(ui);
				}
			});

		egui::CentralPanel::default().show(ctx, |ui| {
			if self.loading {
				ui.spinner();
				ui.label("Loading character data...");
			} else if let Some(error) = &self.error {
				ui.colored_label(egui::Color32::RED, error);
			} else if self.selected_character.is_none() {
				ui.label("Select a character from the list");
			} else {
				self.render_character_details(ui);
			}
		});

		ctx.request_repaint();
	}
}

fn main() -> Result<(), eframe::Error> {
	let options = eframe::NativeOptions::default();

	eframe::run_native(
		"Genshin Character Viewer",
		options,
		Box::new(|cc| Box::new(MyApp::new(cc))),
	)
}
