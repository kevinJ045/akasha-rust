mod utils;
use colored::Color;
use eframe::{egui, App, CreationContext, Frame};
use egui_extras::RetainedImage;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
mod icons;

struct CharacterIcon {
	texture: RetainedImage,
	loading: bool,
	error: bool,
}

struct StatIcons {
	hp: RetainedImage,
	atk: RetainedImage,
	def: RetainedImage,
	crit_rate: RetainedImage,
	crit_dmg: RetainedImage,
	er: RetainedImage,
	em: RetainedImage,
}

impl StatIcons {
	fn new() -> Self {
		Self {
			hp: RetainedImage::from_image_bytes("hp_icon", include_bytes!("../assets/icons/hp.png")).unwrap(),
			atk: RetainedImage::from_image_bytes("atk_icon", include_bytes!("../assets/icons/atk.png")).unwrap(),
			def: RetainedImage::from_image_bytes("def_icon", include_bytes!("../assets/icons/def.png")).unwrap(),
			crit_rate: RetainedImage::from_image_bytes("crit_rate_icon", include_bytes!("../assets/icons/crit_rate.png")).unwrap(),
			crit_dmg: RetainedImage::from_image_bytes("crit_dmg_icon", include_bytes!("../assets/icons/crit_dmg.png")).unwrap(),
			er: RetainedImage::from_image_bytes("er_icon", include_bytes!("../assets/icons/er.png")).unwrap(),
			em: RetainedImage::from_image_bytes("em_icon", include_bytes!("../assets/icons/em.png")).unwrap(),
		}
	}
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
	stat_icons: StatIcons,
	uid: Option<String>,
	uid_input: String,
}

impl MyApp {
	fn new(cc: &CreationContext) -> Self {
		let (tx, rx) = channel();
		let rt = tokio::runtime::Runtime::new().unwrap();
		
		// Try to load saved UID
		let uid = std::fs::read_to_string("saved_uid.txt").ok();
		
		let mut app = Self {
			characters: None,
			calculations: None,
			selected_character: None,
			loading: false,
			error: None,
			rx,
			tx: tx.clone(),
			icons: Arc::new(Mutex::new(HashMap::new())),
			runtime: Arc::new(rt),
			stat_icons: StatIcons::new(),
			uid,
			uid_input: String::new(),
		};
		
		// If we have a saved UID, load the data
		if app.uid.is_some() {
			app.load_data();
		}
		
		app
	}

	fn load_data(&mut self) {
		if let Some(uid) = &self.uid {
			self.loading = true;
			let uid_clone = uid.clone();
			let rt = tokio::runtime::Runtime::new().unwrap();
			let tx = self.tx.clone();

			std::thread::spawn(move || {
				rt.block_on(async {
					match (
						utils::get_user_builds(&uid_clone).await,
						utils::get_user_calculations(&uid_clone).await,
					) {
						(Ok(chars), Ok(calcs)) => {
							if let Some(char_array) = chars.as_array() {
								tx.send(Ok((char_array.to_vec(), calcs))).unwrap();
							}
						}
						_ => tx
							.send(Err("Failed to fetch data".to_string()))
							.unwrap(),
					}
				});
			});
			
			// self.runtime.spawn(async move {
			// 	match (
			// 		utils::get_user_builds(&uid_clone).await,
			// 		utils::get_user_calculations(&uid_clone).await,
			// 	) {
			// 		(Ok(chars), Ok(calcs)) => {
			// 			if let Some(char_array) = chars.as_array() {
			// 				tx.send(Ok((char_array.to_vec(), calcs))).unwrap();
			// 			}
			// 		}
			// 		_ => tx.send(Err("Failed to fetch data".to_string())).unwrap(),
			// 	}
			// });
		}
	}

	async fn refresh_data(&self) -> Result<(), Box<dyn std::error::Error>> {
		if let Some(uid) = &self.uid {
			let url = format!("https://akasha.cv/api/user/refresh/{}", uid);
			reqwest::get(&url).await?;
		}
		Ok(())
	}

	fn save_uid(&self) {
		if let Some(uid) = &self.uid {
			std::fs::write("saved_uid.txt", uid).ok();
		}
	}

	fn logout(&mut self) {
		self.uid = None;
		self.characters = None;
		self.calculations = None;
		self.selected_character = None;
		self.uid_input.clear();
		std::fs::remove_file("saved_uid.txt").ok();
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

	fn render_constellations(&self, ui: &mut egui::Ui, short_name: &str, constellation: i64) {
		ui.vertical(|ui| {
			for i in 1..=6 {
				let cons_url = format!("https://enka.network/ui/UI_Talent_S_{}_{:02}.png", short_name, i);
				self.ensure_icon(&cons_url);

				if let Some(icon) = self.icons.lock().unwrap().get(&cons_url) {
					let size = 48.0;
					
					if i <= constellation {
						icon.texture.show_size(ui, egui::vec2(size, size));
					}
				}
				ui.add_space(4.0); // Small space between constellation icons
			}
		});
	}

	fn find_by_character_id(&self, target_id: i64) -> Option<&Value> {
		self.calculations.as_ref().and_then(|calcs| {
			calcs.as_array()?.iter().find(|calc| {
				calc["characterId"].as_i64() == Some(target_id)
			})
		})
	}

	fn render_character_details(&self, ui: &mut egui::Ui) {
		if let Some(idx) = self.selected_character {
			if let Some(chars) = &self.characters {
				if let Some(char) = chars.get(idx) {
					// Get character name and constellation level
					let name = char["name"].as_str().unwrap_or("Unknown");
					let constellation = char["constellation"].as_i64().unwrap_or(0);
					let short_name = char.get("icon")
						.and_then(|i| i.as_str())
						.unwrap()
						.rsplit('_')
						.next()
						.and_then(|part| part.split('.').next())
						.unwrap();

					// println!("{}", serde_json::to_string_pretty(&char).unwrap());

					// Create right-side overlay for constellations and name
					let screen_rect = ui.max_rect();
					
					// Constellation panel - center-right
					let cons_width = 64.0;
					let cons_rect = egui::Rect::from_min_max(
						egui::pos2(screen_rect.right() - cons_width - 20.0, screen_rect.center().y - 180.0),
						egui::pos2(screen_rect.right() - 20.0, screen_rect.center().y + 180.0),
					);
					
					// Name panel - bottom-right
					let name_height = 40.0;
					let name_rect = egui::Rect::from_min_max(
						egui::pos2(screen_rect.right() - 300.0, screen_rect.bottom() - name_height - 20.0),
						egui::pos2(screen_rect.right() - 20.0, screen_rect.bottom() - 20.0),
					);
					let ranking_height = 40.0;
					let ranking_rect = egui::Rect::from_min_max(
						egui::pos2(screen_rect.right() - 300.0, screen_rect.top() + ranking_height + 20.0),
						egui::pos2(screen_rect.right() - 20.0, screen_rect.bottom() - 20.0),
					);

					// Render constellations
					let cons_response = egui::Area::new("constellations")
						.fixed_pos(cons_rect.min)
						.show(ui.ctx(), |ui| {
							self.render_constellations(ui, short_name, constellation);
						});

					// Render name
					egui::Area::new("character_name")
						.fixed_pos(name_rect.min)
						.show(ui.ctx(), |ui| {
							ui.heading(egui::RichText::new(name)
								.size(32.0)
								.strong());
						});

					// Render Ranking
					egui::Area::new("ranking")
						.fixed_pos(ranking_rect.min)
						.show(ui.ctx(), |ui| {
							// println!("Character data: {:#?}", char);  // Debug print the entire character data
							
							if let Some(char_id) = char["characterId"].as_i64() {
								if let Some(calculation) = self.find_by_character_id(char_id) {
									// println!("Found calculation: {:#?}", calculation);
									
									if let Some(calc) = calculation.get("calculations").and_then(|c| c.get("fit")).and_then(|f| f.as_object()) {
										// println!("{:#?}", calc);
										if let (Some(rank), Some(total)) = (
											calc.get("ranking").and_then(|v| v.as_i64()),
											calc.get("outOf").and_then(|v| v.as_i64())
										) {
											let percentage = (rank as f64 / total as f64 * 100.0) as i64;
											ui.heading(egui::RichText::new(
												format!("Top {}% ({}/{})", percentage, rank, total)
											)
												.size(22.0)
												.strong());
										}
									}
								} else {
									println!("No calculation found for character ID: {}", char_id);
								}
							} else {
								println!("Could not find characterId as number in: {:#?}", char.get("characterId"));
							}
						});

					let element = char["characterMetadata"]["element"]
						.as_str()
						.unwrap_or("")
						.to_lowercase();
					
					let bg_url = format!("https://akasha.cv/elementalBackgrounds/{}-bg.jpg", 
						element.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default() + &element[1..]);
					self.ensure_icon(&bg_url);

					let rect = ui.available_rect_before_wrap();
					
					// Paint background if available
					if let Some(bg) = self.icons.lock().unwrap().get(&bg_url) {
						if !bg.loading && !bg.error {
							let img_size = bg.texture.size_vec2();
							let img_aspect = img_size.x / img_size.y;
							let rect_aspect = rect.width() / rect.height();
							
							let uv_rect = if img_aspect > rect_aspect {
								// Image is wider than container - crop sides
								let uv_width = rect_aspect / img_aspect;
								let uv_x = (1.0 - uv_width) / 2.0;
								egui::Rect::from_min_max(
									egui::pos2(uv_x, 0.0),
									egui::pos2(uv_x + uv_width, 1.0)
								)
							} else {
								// Image is taller than container - crop top/bottom
								let uv_height = img_aspect / rect_aspect;
								let uv_y = (1.0 - uv_height) / 2.0;
								egui::Rect::from_min_max(
									egui::pos2(0.0, uv_y),
									egui::pos2(1.0, uv_y + uv_height)
								)
							};

							ui.painter().image(
								bg.texture.texture_id(ui.ctx()),
								rect,
								uv_rect,
								egui::Color32::WHITE,
							);
							
							// Add overlay
							ui.painter().rect_filled(
								rect,
								0.0,
								egui::Color32::from_black_alpha(180),
							);
						}
					}

					if let Some(name) = char.get("icon").and_then(|i| i.as_str()) {
						let icon_url = format!("https://enka.network/ui/UI_Gacha_AvatarImg_{}.png", short_name);
						self.ensure_icon(&icon_url);

						if let Some(icon) = self.icons.lock().unwrap().get(&icon_url) {
							if !icon.loading && !icon.error {
								// let rect = ui.available_rect_before_wrap();
								let img_size = icon.texture.size_vec2();
								let img_aspect = img_size.x / img_size.y;
								let rect_aspect = rect.width() / rect.height();
								
								let uv_rect = if img_aspect > rect_aspect {
									// Image is wider than container - crop sides
									let uv_width = rect_aspect / img_aspect;
									let uv_x = (1.0 - uv_width) / 2.0;
									egui::Rect::from_min_max(
										egui::pos2(uv_x, 0.0),
										egui::pos2(uv_x + uv_width, 1.0)
									)
								} else {
									// Image is taller than container - crop top/bottom
									let uv_height = img_aspect / rect_aspect;
									let uv_y = (1.0 - uv_height) / 2.0;
									egui::Rect::from_min_max(
										egui::pos2(0.0, uv_y),
										egui::pos2(1.0, uv_y + uv_height)
									)
								};

								ui.painter().image(
									icon.texture.texture_id(ui.ctx()),
									rect,
									uv_rect,
									egui::Color32::WHITE,
								);
							} else if icon.loading {
								ui.spinner();
								ui.label("Loading character art...");
							} else {
								ui.label("Failed to load character art");
							}
						}
					}

					let full_height = ui.available_height();

					// Continue with existing UI
					ui.horizontal(|ui| {
						// Left panel for avatar (40% width)

						// Right panel for character details (60% width)
						egui::Frame::none()
							.inner_margin(10.0)
							.show(ui, |ui| {
								ui.vertical(|ui| {
										// Card 1: Character Info and Talents
										{
											egui::Frame::none()
												// .fill(ui.style().visuals.extreme_bg_color)
												// .rounding(10.0)
												// .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.inner_margin(10.0)
												.show(ui, |ui| {
													ui.set_min_width(200.0);
													ui.vertical(|ui| {
														ui.heading("Character Info");
														ui.label(format!(
															"Level {}/90",
															char["propMap"]["level"]["val"].as_str().unwrap()
														));
														ui.label(format!("Constellation: C{}", char["constellation"].as_i64().unwrap_or(0)));

														ui.add_space(10.0);
														ui.heading("Talents");
														// First get the normal attack icon based on weapon type
														let normal_attack_icon = match char["weaponType"].as_str().unwrap_or("") {
															"WeAPON_SWORD_ONE_HAND" => "Skill_A_01",
															"WeAPON_BOW" => "Skill_A_02",
															"WeAPON_POLE" => "Skill_A_03",
															"WeAPON_CLAYMORE" => "Skill_A_04",
															"WeAPON_CATALYST" => "Skill_A_05",
															_ => "Skill_A_01", // default to sword if unknown
														};

														// Generate URLs for all three talent icons
														let normal_attack_url = format!("https://enka.network/ui/{}.png", normal_attack_icon);
														let skill_url = format!("https://enka.network/ui/Skill_S_{}_01.png", short_name);
														let burst_url = format!("https://enka.network/ui/Skill_E_{}_01.png", short_name);

														// Ensure all talent icons are loaded
														self.ensure_icon(&normal_attack_url);
														self.ensure_icon(&skill_url);
														self.ensure_icon(&burst_url);

														ui.horizontal(|ui| {
															if let Some(icon) = self.icons.lock().unwrap().get(&normal_attack_url) {
																let size = 32.0;
																icon.texture.show_size(ui, egui::vec2(size, size));
															}
															ui.label(format!(
																"Normal Attack: {}",
																char["talentsLevelMap"]["normalAttacks"]["level"].as_i64().unwrap_or(0)
															));
														});

														ui.horizontal(|ui| {
															if let Some(icon) = self.icons.lock().unwrap().get(&skill_url) {
																let size = 32.0;
																icon.texture.show_size(ui, egui::vec2(size, size));
															}
															ui.label(format!(
																"Elemental Skill: {}",
																char["talentsLevelMap"]["elementalSkill"]["level"].as_i64().unwrap_or(0)
															));
														});

														ui.horizontal(|ui| {
															if let Some(icon) = self.icons.lock().unwrap().get(&burst_url) {
																let size = 32.0;
																icon.texture.show_size(ui, egui::vec2(size, size));
															}
															ui.label(format!(
																"Elemental Burst: {}",
																char["talentsLevelMap"]["elementalBurst"]["level"].as_i64().unwrap_or(0)
															));
														});
													});
												});
										}

										// Card 2: Weapon Info
										{
											egui::Frame::none()
												// .fill(ui.style().visuals.extreme_bg_color)
												// .rounding(10.0)
												.inner_margin(10.0)
												// .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
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
																"Level {}/90",
																weapon["weaponInfo"]["level"].as_i64().unwrap_or(0)
															));
														}
													});
												});
										}

										ui.end_row();

										// Card 3: Stats
										{
											egui::Frame::none()
												// .fill(ui.style().visuals.extreme_bg_color)
												// .rounding(10.0)
												.inner_margin(10.0)
												// .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.show(ui, |ui| {
													ui.vertical(|ui| {
														ui.heading("Stats");
														
														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.hp.show_size(ui, egui::vec2(size, size));
															ui.label(format!("HP | {}", utils::format_number(char["stats"]["maxHp"]["value"].as_f64().unwrap_or(0.0))));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.atk.show_size(ui, egui::vec2(size, size));
															ui.label(format!("ATK | {}", utils::format_number(char["stats"]["atk"]["value"].as_f64().unwrap_or(0.0))));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.def.show_size(ui, egui::vec2(size, size));
															ui.label(format!("DEF | {}", utils::format_number(char["stats"]["def"]["value"].as_f64().unwrap_or(0.0))));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.crit_rate.show_size(ui, egui::vec2(size, size));
															ui.label(format!("Crit Rate | {:.1}%", char["stats"]["critRate"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.crit_dmg.show_size(ui, egui::vec2(size, size));
															ui.label(format!("Crit DMG | {:.1}%", char["stats"]["critDamage"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.er.show_size(ui, egui::vec2(size, size));
															ui.label(format!("Energy Recharge | {:.1}%", char["stats"]["energyRecharge"]["value"].as_f64().unwrap_or(0.0) * 100.0));
														});

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.em.show_size(ui, egui::vec2(size, size));
															ui.label(format!("Elemental Mastery | {}", char["stats"]["elementalMastery"]["value"].as_f64().unwrap_or(0.0) as i64));
														});
													});
												});
										}

										// Card 4: Artifacts and Build Quality
										{
											egui::Frame::none()
												// .fill(ui.style().visuals.extreme_bg_color)
												// .rounding(10.0)
												.inner_margin(10.0)
												// .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
												.show(ui, |ui| {
													ui.vertical(|ui| {

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
														

														ui.horizontal(|ui| {
															let size = 16.0;
															self.stat_icons.crit_dmg.show_size(ui, egui::vec2(size, size));
															ui.label(format!("Crit Value | {:.2}", char["critValue"].as_f64().unwrap_or(0.0)));
														});
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

impl App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
		if self.uid.is_none() {
			// Show UID input dialog
			egui::CentralPanel::default().show(ctx, |ui| {
				ui.vertical_centered(|ui| {
					ui.add_space(100.0);
					ui.heading("Enter your Genshin Impact UID");
					ui.add_space(20.0);
					
					let text_edit = ui.add(egui::TextEdit::singleline(&mut self.uid_input)
						.hint_text("Enter UID...")
						.desired_width(200.0));
					
					if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
						if !self.uid_input.is_empty() {
							self.uid = Some(self.uid_input.clone());
							self.save_uid();
							self.load_data();
						}
					}
				});
			});
			return;
		}

		// Handle loading results
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

		// Main UI
		egui::SidePanel::left("character_list")
			.default_width(200.0)
			.show(ctx, |ui| {
				ui.vertical(|ui| {
					ui.heading("Characters");
					
					// Add refresh and logout buttons
					if ui.button("🔄 Refresh").clicked() {
						let runtime = self.runtime.clone();
						let tx = self.tx.clone();
						let rt = tokio::runtime::Runtime::new().unwrap();
						if let Some(uid) = &self.uid {
							let uid_clone = uid.clone();
							self.loading = true;
							// let tx = self.tx.clone();

							std::thread::spawn(move || {
								rt.block_on(async {
									match (
										utils::get_user_builds(&uid_clone).await,
										utils::get_user_calculations(&uid_clone).await,
									) {
										(Ok(chars), Ok(calcs)) => {
											if let Some(char_array) = chars.as_array() {
												tx.send(Ok((char_array.to_vec(), calcs))).unwrap();
											}
										}
										_ => tx
											.send(Err("Failed to fetch data".to_string()))
											.unwrap(),
									}
								});
							});
							// runtime.spawn(async move {
							// 	match utils::refresh_user(&uid_clone).await {
							// 		Ok(()) => {
							// 			// Wait a bit for the refresh to take effect
							// 			tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
										
							// 			match (
							// 				utils::get_user_builds(&uid_clone).await,
							// 				utils::get_user_calculations(&uid_clone).await,
							// 			) {
							// 				(Ok(chars), Ok(calcs)) => {
							// 					if let Some(char_array) = chars.as_array() {
							// 						tx.send(Ok((char_array.to_vec(), calcs))).unwrap();
							// 					}
							// 				}
							// 				_ => tx.send(Err("Failed to fetch data".to_string())).unwrap(),
							// 			}
							// 		}
							// 		Err(e) => tx.send(Err(format!("Failed to refresh: {}", e))).unwrap(),
							// 	}
							// });
						}
					}
					
					if ui.button("🚪 Logout").clicked() {
						self.logout();
						return;
					}
					
					ui.separator();
					
					if self.loading {
						ui.spinner();
					} else if let Some(error) = &self.error {
						ui.colored_label(egui::Color32::RED, error);
					} else {
						self.render_character_list(ui);
					}
				});
			});

		// Then render the panels on top
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
