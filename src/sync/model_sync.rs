use std::collections::{HashMap, HashSet};

use ankit::CreateModelParams;
use eyre::Result;
use tracing::{info, warn};

use crate::sync::client::{FlashClient, unique_name};
use crate::sync::identity::make_model_uuid_comment;

pub struct ModelSyncData {
	pub uuid:       uuid::Uuid,
	pub name:       String,
	pub fields:     Vec<String>,
	pub templates:  Vec<TemplateSyncData>,
	pub css:        String,
	pub latex_pre:  Option<String>,
	pub latex_post: Option<String>,
}

pub struct TemplateSyncData {
	pub name:  String,
	pub front: String,
	pub back:  String,
}

pub async fn sync_model(client: &FlashClient, model: &ModelSyncData) -> Result<String> {
	let existing_model_name = find_model_by_uuid(client, &model.uuid).await?;

	let model_name = if let Some(existing_name) = existing_model_name {
		info!("Model '{}' exists with uuid, updating", existing_name);
		update_model_if_changed(client, model, &existing_name).await?;
		existing_name
	} else {
		info!("Creating new model '{}'", model.name);
		create_model(client, model).await?
	};

	Ok(model_name)
}

async fn find_model_by_uuid(client: &FlashClient, uuid: &uuid::Uuid) -> Result<Option<String>> {
	let model_names = client.models().names().await.map_err(|e| {
		eyre::eyre!("Failed to get model names: {}", e)
	})?;

	let uuid_str = uuid.to_string();
	let comment_marker = format!("/* flash-uuid: {}", uuid_str);

	for name in &model_names {
		let styling = client.models().styling(name).await.map_err(|e| {
			eyre::eyre!("Failed to get styling for model '{}': {}", name, e)
		})?;

		if styling.css.contains(&comment_marker) || styling.css.contains(&uuid_str) {
			return Ok(Some(name.clone()));
		}
	}

	Ok(None)
}

async fn create_model(client: &FlashClient, model: &ModelSyncData) -> Result<String> {
	let existing_names: HashSet<String> = client.models().names().await.map_err(|e| {
		eyre::eyre!("Failed to get model names: {}", e)
	})?.into_iter().collect();

	let target_name = unique_name(&model.name, &existing_names);

	let mut params = CreateModelParams::new(&target_name);
	for field_name in &model.fields {
		params = params.field(field_name);
	}
	params = params.css(&model_css_with_uuid(model));
	for tmpl in &model.templates {
		params = params.template(&tmpl.name, &tmpl.front, &tmpl.back);
	}

	client.models().create(params).await.map_err(|e| {
		eyre::eyre!("Failed to create model '{}': {}", target_name, e)
	})?;

	client
		.models()
		.update_styling(&target_name, &model_css_with_uuid(model))
		.await
		.map_err(|e| eyre::eyre!("Failed to update model styling: {}", e))?;

	info!("Created model '{}'", target_name);
	Ok(target_name)
}

async fn update_model_if_changed(
	client: &FlashClient,
	model: &ModelSyncData,
	existing_name: &str,
) -> Result<()> {
	let styling = client.models().styling(existing_name).await.map_err(|e| {
		eyre::eyre!("Failed to get styling for model '{}': {}", existing_name, e)
	})?;

	let desired_css = model_css_with_uuid(model);
	if styling.css != desired_css {
		info!("Updating CSS for model '{}'", existing_name);
		client
			.models()
			.update_styling(existing_name, &desired_css)
			.await
			.map_err(|e| eyre::eyre!("Failed to update styling: {}", e))?;
	}

	let existing_templates = client.models().templates(existing_name).await.map_err(|e| {
		eyre::eyre!("Failed to get templates for model '{}': {}", existing_name, e)
	})?;

	for tmpl in &model.templates {
		let needs_update = match existing_templates.get(&tmpl.name) {
			None => true,
			Some(existing) => existing.front != tmpl.front || existing.back != tmpl.back,
		};

		if needs_update {
			info!("Updating template '{}' for model '{}'", tmpl.name, existing_name);
			let mut updates: HashMap<&str, (&str, &str)> = HashMap::new();
			updates.insert(&tmpl.name, (&tmpl.front, &tmpl.back));
			client
				.models()
				.update_templates(existing_name, updates)
				.await
				.map_err(|e| eyre::eyre!("Failed to update templates: {}", e))?;
		}
	}

	let existing_fields = client.models().field_names(existing_name).await.map_err(|e| {
		eyre::eyre!("Failed to get field names: {}", e)
	})?;

	for field_name in &model.fields {
		if !existing_fields.contains(field_name) {
			warn!("Field '{}' missing from model '{}', adding", field_name, existing_name);
			client
				.models()
				.add_field(existing_name, field_name, None)
				.await
				.map_err(|e| eyre::eyre!("Failed to add field '{}': {}", field_name, e))?;
		}
	}

	for existing_field in &existing_fields {
		if !model.fields.contains(existing_field) {
			warn!(
				"Field '{}' exists in Anki model '{}' but not in flash. Removal skipped to avoid data loss.",
				existing_field, existing_name
			);
		}
	}

	Ok(())
}

fn model_css_with_uuid(model: &ModelSyncData) -> String {
	let uuid_comment = make_model_uuid_comment(&model.uuid);
	format!("{}\n{}", uuid_comment, model.css)
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::{model_css_with_uuid, ModelSyncData, TemplateSyncData};

	fn sample_model() -> ModelSyncData {
		ModelSyncData {
			uuid:       Uuid::from_u128(42),
			name:       "Test Model".into(),
			fields:     vec!["Front".into(), "Back".into()],
			templates:  vec![TemplateSyncData {
				name:  "Card 1".into(),
				front: "{{Front}}".into(),
				back:  "{{Back}}".into(),
			}],
			css:        ".card { color: red; }".into(),
			latex_pre:  None,
			latex_post: None,
		}
	}

	#[test]
	fn model_css_includes_uuid_comment() {
		let model = sample_model();
		let css = model_css_with_uuid(&model);
		assert!(css.contains("/* flash-uuid:"));
		assert!(css.contains(&model.uuid.to_string()));
		assert!(css.ends_with(".card { color: red; }"));
	}

	#[test]
	fn model_css_with_empty_original_css() {
		let model = ModelSyncData {
			css: String::new(),
			..sample_model()
		};
		let css = model_css_with_uuid(&model);
		assert!(css.contains(&model.uuid.to_string()));
		assert!(css.ends_with('\n'));
	}

	#[test]
	fn model_css_uuid_comment_starts_on_first_line() {
		let model = sample_model();
		let css = model_css_with_uuid(&model);
		let first_line = css.lines().next().unwrap();
		assert!(first_line.starts_with("/* flash-uuid:"));
		assert!(first_line.ends_with("*/"));
	}

	#[test]
	fn model_css_preserves_original_content() {
		let model = sample_model();
		let css = model_css_with_uuid(&model);
		assert!(css.contains(".card"));
		assert!(css.contains("color: red"));
	}

	#[test]
	fn model_css_uuid_is_parseable() {
		let model = sample_model();
		let css = model_css_with_uuid(&model);
		let parsed = crate::sync::identity::parse_model_uuid_from_css(&css);
		assert_eq!(parsed, Some(model.uuid));
	}
}
