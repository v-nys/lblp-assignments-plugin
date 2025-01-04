use extism_pdk::*;
use logic_based_learning_paths::domain_without_loading::{ArtifactMapping, ExtensionFieldProcessingPayload, ExtensionFieldProcessingResult, NodeProcessingPayload, NodeProcessingError};
use schemars::JsonSchema;
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use logic_based_learning_paths::prelude::*;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone, JsonSchema)]
struct Assignment {
    id: String,
    title: Option<String>,
    // TODO: vermelden dat dit altijd relatief en met fwd slashes moet!
    // anders lastig als iemand op Windows schrijft en UNIX gebruikt of vice versa
    attachments: Option<Vec<String>>,
}

fn file_is_readable(file_path: &Path) -> bool {
    file_path.is_file() && File::open(file_path).is_ok()
}

/*
#[plugin_fn]
pub fn get_extension_field_schema(
    _node_processing_payload: NodeProcessingPayload,
) -> FnResult<HashMap<(String, bool), serde_json::Value>> {
    let mut map = HashMap::new();
    let _ = map.insert(
        ("assignments".into(), false),
        serde_json::to_value(schema_for!(Vec<Assignment>)).expect("Generating a schema for this type works."),
    );
    Ok(map)
}
*/


#[plugin_fn]
pub fn process_extension_field(
    ExtensionFieldProcessingPayload {node_processing_payload, field_name, value}: ExtensionFieldProcessingPayload,
) -> FnResult<ExtensionFieldProcessingResult> {
    let NodeProcessingPayload { node, cluster_path, parameter_values } = node_processing_payload;
    if field_name != "assignments" {
        Ok(ExtensionFieldProcessingResult { result: Err(NodeProcessingError::CannotProcessFieldType) })
    } else {
        let assignments = serde_yaml::from_value::<Vec<Assignment>>(value.clone());
        match assignments {
            Ok(assignments) => {
                let mut additional_remarks = vec![];
                let mut artifacts = HashSet::new();
                let assignment_ids: HashSet<_> = assignments.iter().map(|a| &a.id).collect();
                if assignment_ids.len() < assignments.len() {
                    additional_remarks
                        .push(format!("Duplicate assignment IDs in node {}", node.node_id));
                }
                assignments.iter().for_each(|assignment| {
                    let base_assignment_path = cluster_path.join(node.node_id.local_id.clone()).join(&assignment.id);
                    let contents_path = base_assignment_path.join("contents.html");
                    if !file_is_readable(&contents_path) {
                        additional_remarks.push(
                            format!("Assignment {} associated with node {} lacks a readable contents.html file.", 
                                    &assignment.id,
                                    node.node_id.local_id)
                            );
                    }
                    else {
                        artifacts.insert(ArtifactMapping {
                            local_file: contents_path,
                            root_relative_target_dir: PathBuf::from(format!("{}/{}/assignments/{}", &node.node_id.namespace, &node.node_id.local_id, &assignment.id))
                        });
                        
                    }
                    if let Some(attachments) = assignment.attachments.as_ref() {
                        attachments.iter().for_each(|attachment| {
                            let attachment_path = base_assignment_path.join(attachment);
                            if !file_is_readable(attachment_path.as_path()) {
                                additional_remarks.push(format!("Attachment cannot be read at {}", attachment_path.to_string_lossy()));
                            }
                            else {
                        artifacts.insert(
                            ArtifactMapping {
                                local_file: attachment_path,
                                root_relative_target_dir: PathBuf::from(format!("{}/{}/assignments/{}/attachments", &node.node_id.namespace, &node.node_id.local_id, &assignment.id))
                            });
                            }
                        });
                    }
                });
                // attachments should be present and readable
                if additional_remarks.is_empty() {
                    Ok(ExtensionFieldProcessingResult { result: Ok(artifacts) })
                } else {
                    Ok(ExtensionFieldProcessingResult { result: Err(NodeProcessingError::Remarks(additional_remarks)) })
                }
            }
            Err(e) => 
                Ok(ExtensionFieldProcessingResult { result: Err(NodeProcessingError::Remarks(vec![format!(
                "Something went wrong while deserializing assignments in node {}: {}",
                node.node_id,
                e.to_string()
            )]))}),
        }
    }
}
