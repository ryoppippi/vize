use super::{errors::compact_error, ActiveProject, CorsaTypeAwareSession, TypeProbe};
use corsa::{api::TypeResponse, runtime::block_on};
use vize_carton::{CompactString, String, ToCompactString};

impl CorsaTypeAwareSession {
    pub(in crate::linter) fn probe_type_at_offset(
        &self,
        active_project: &ActiveProject,
        generated_source: &str,
        generated_offset: u32,
        load_property_types: bool,
        load_signatures: bool,
    ) -> Result<Option<TypeProbe>, String> {
        let utf16_offset = byte_offset_to_utf16_offset(generated_source, generated_offset);
        let symbol = block_on(self.client.get_symbol_at_position(
            active_project.snapshot.handle.clone(),
            active_project.project.clone(),
            self.virtual_file_wire.as_str(),
            utf16_offset,
        ))
        .map_err(|error| {
            compact_error(
                "Failed to query checker symbol",
                error.to_compact_string().as_str(),
            )
        })?;
        let type_response = if let Some(symbol) = symbol {
            match block_on(self.client.get_type_of_symbol(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                symbol.id,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker symbol type",
                    error.to_compact_string().as_str(),
                )
            })? {
                Some(type_response) => Some(type_response),
                None => block_on(self.client.get_type_at_position(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    self.virtual_file_wire.as_str(),
                    utf16_offset,
                ))
                .map_err(|error| {
                    compact_error(
                        "Failed to query checker type",
                        error.to_compact_string().as_str(),
                    )
                })?,
            }
        } else {
            block_on(self.client.get_type_at_position(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                self.virtual_file_wire.as_str(),
                utf16_offset,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker type",
                    error.to_compact_string().as_str(),
                )
            })?
        };
        let Some(type_response) = type_response else {
            return Ok(None);
        };

        let mut probe = TypeProbe {
            type_texts: self.collect_type_texts(active_project, &type_response)?,
            property_names: Vec::new(),
            property_types: Vec::new(),
            call_signatures: Vec::new(),
            return_types: Vec::new(),
        };

        let properties = block_on(self.client.get_properties_of_type(
            active_project.snapshot.handle.clone(),
            active_project.project.clone(),
            type_response.id.clone(),
        ))
        .map_err(|error| {
            compact_error(
                "Failed to query checker properties",
                error.to_compact_string().as_str(),
            )
        })?;

        probe.property_names.reserve(properties.len());
        for property in &properties {
            probe
                .property_names
                .push(property.name.as_str().to_compact_string());
        }

        if load_property_types && !properties.is_empty() {
            let property_types = block_on(
                self.client.get_types_of_symbols(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    properties
                        .iter()
                        .map(|property| property.id.clone())
                        .collect(),
                ),
            )
            .map_err(|error| {
                compact_error(
                    "Failed to query checker property types",
                    error.to_compact_string().as_str(),
                )
            })?;

            probe.property_types.reserve(property_types.len());
            for property_type in property_types {
                if let Some(property_type) = property_type {
                    probe
                        .property_types
                        .push(self.collect_type_texts(active_project, &property_type)?);
                } else {
                    probe.property_types.push(Vec::new());
                }
            }
        }

        if load_signatures {
            let signatures = block_on(self.client.get_signatures_of_type(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                type_response.id,
                0,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to query checker signatures",
                    error.to_compact_string().as_str(),
                )
            })?;

            probe.call_signatures.reserve(signatures.len());
            probe.return_types.reserve(signatures.len());
            for signature in signatures {
                if signature.parameters.is_empty() {
                    probe.call_signatures.push(Vec::new());
                } else {
                    let parameter_types = block_on(self.client.get_types_of_symbols(
                        active_project.snapshot.handle.clone(),
                        active_project.project.clone(),
                        signature.parameters.clone(),
                    ))
                    .map_err(|error| {
                        compact_error(
                            "Failed to query checker signature parameter types",
                            error.to_compact_string().as_str(),
                        )
                    })?;

                    let mut rendered_parameters = Vec::with_capacity(parameter_types.len());
                    for parameter_type in parameter_types {
                        if let Some(parameter_type) = parameter_type {
                            rendered_parameters
                                .push(self.collect_type_texts(active_project, &parameter_type)?);
                        } else {
                            rendered_parameters.push(Vec::new());
                        }
                    }
                    probe.call_signatures.push(rendered_parameters);
                }

                let return_type = block_on(self.client.get_return_type_of_signature(
                    active_project.snapshot.handle.clone(),
                    active_project.project.clone(),
                    signature.id,
                ))
                .map_err(|error| {
                    compact_error(
                        "Failed to query checker signature return type",
                        error.to_compact_string().as_str(),
                    )
                })?;

                if let Some(return_type) = return_type {
                    probe
                        .return_types
                        .push(self.collect_type_texts(active_project, &return_type)?);
                } else {
                    probe.return_types.push(Vec::new());
                }
            }
        }

        Ok(Some(probe))
    }

    fn collect_type_texts(
        &self,
        active_project: &ActiveProject,
        type_response: &TypeResponse,
    ) -> Result<Vec<CompactString>, String> {
        let mut texts = Vec::with_capacity(type_response.texts.len() + 1);
        for text in &type_response.texts {
            push_unique_text(&mut texts, text.as_str());
        }
        if texts.is_empty() {
            let rendered = block_on(self.client.type_to_string(
                active_project.snapshot.handle.clone(),
                active_project.project.clone(),
                type_response.id.clone(),
                None,
                None,
            ))
            .map_err(|error| {
                compact_error(
                    "Failed to render checker type",
                    error.to_compact_string().as_str(),
                )
            })?;
            push_unique_text(&mut texts, rendered.as_str());
        }
        Ok(texts)
    }
}

pub(super) fn byte_offset_to_utf16_offset(source: &str, byte_offset: u32) -> u32 {
    let mut clamped = usize::min(byte_offset as usize, source.len());
    while clamped > 0 && !source.is_char_boundary(clamped) {
        clamped -= 1;
    }
    source[..clamped].encode_utf16().count() as u32
}

fn push_unique_text(texts: &mut Vec<CompactString>, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    if texts.iter().any(|existing| existing.as_str() == trimmed) {
        return;
    }
    texts.push(trimmed.to_compact_string());
}
