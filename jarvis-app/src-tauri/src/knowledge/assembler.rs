use std::path::Path;
use crate::knowledge::store::GemMeta;

/// Format content section (no heading - will be added by assembler)
pub fn format_content(_title: &str, content: &str) -> String {
    content.to_string()
}

/// Format enrichment section from JSON
pub fn format_enrichment(enrichment: &serde_json::Value) -> String {
    let mut output = String::new();

    // Summary section
    if let Some(summary) = enrichment.get("summary").and_then(|v| v.as_str()) {
        if !summary.is_empty() {
            output.push_str("## Summary\n\n");
            output.push_str(summary);
            output.push_str("\n\n");
        }
    }

    // Tags section
    if let Some(tags) = enrichment.get("tags").and_then(|v| v.as_array()) {
        if !tags.is_empty() {
            output.push_str("## Tags\n\n");
            for tag in tags {
                if let Some(tag_str) = tag.as_str() {
                    output.push_str("- ");
                    output.push_str(tag_str);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
    }

    // Enrichment Metadata section
    let provider = enrichment.get("provider").and_then(|v| v.as_str());
    let enriched_at = enrichment.get("enriched_at").and_then(|v| v.as_str());
    
    if provider.is_some() || enriched_at.is_some() {
        output.push_str("## Enrichment Metadata\n\n");
        if let Some(p) = provider {
            output.push_str("- Provider: ");
            output.push_str(p);
            output.push('\n');
        }
        if let Some(e) = enriched_at {
            output.push_str("- Enriched: ");
            output.push_str(e);
            output.push('\n');
        }
        output.push('\n');
    }

    output
}

/// Format transcript section
pub fn format_transcript(transcript: &str, language: &str) -> String {
    format!("## Transcript\n\nLanguage: {}\n\n{}", language, transcript)
}

/// Format co-pilot analysis section
pub fn format_copilot(copilot_data: &serde_json::Value) -> String {
    let mut output = String::new();

    // Rolling Summary
    if let Some(summary) = copilot_data.get("updated_summary")
        .or_else(|| copilot_data.get("summary"))
        .and_then(|v| v.as_str()) 
    {
        if !summary.is_empty() {
            output.push_str("## Rolling Summary\n\n");
            output.push_str(summary);
            output.push_str("\n\n");
        }
    }

    // Key Points
    if let Some(points) = copilot_data.get("key_points").and_then(|v| v.as_array()) {
        if !points.is_empty() {
            output.push_str("## Key Points\n\n");
            for point in points {
                if let Some(p) = point.as_str() {
                    output.push_str("- ");
                    output.push_str(p);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
    }

    // Decisions
    if let Some(decisions) = copilot_data.get("decisions").and_then(|v| v.as_array()) {
        if !decisions.is_empty() {
            output.push_str("## Decisions\n\n");
            for decision in decisions {
                if let Some(d) = decision.as_str() {
                    output.push_str("- ");
                    output.push_str(d);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
    }

    // Action Items
    if let Some(items) = copilot_data.get("action_items").and_then(|v| v.as_array()) {
        if !items.is_empty() {
            output.push_str("## Action Items\n\n");
            for item in items {
                if let Some(i) = item.as_str() {
                    output.push_str("- ");
                    output.push_str(i);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
    }

    // Open Questions
    if let Some(questions) = copilot_data.get("open_questions").and_then(|v| v.as_array()) {
        if !questions.is_empty() {
            output.push_str("## Open Questions\n\n");
            for question in questions {
                if let Some(q) = question.as_str() {
                    output.push_str("- ");
                    output.push_str(q);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
    }

    // Key Concepts
    if let Some(concepts) = copilot_data.get("key_concepts").and_then(|v| v.as_array()) {
        if !concepts.is_empty() {
            output.push_str("## Key Concepts\n\n");
            for concept in concepts {
                if let Some(obj) = concept.as_object() {
                    let term = obj.get("term").and_then(|v| v.as_str());
                    let context = obj.get("context").and_then(|v| v.as_str());
                    if let (Some(t), Some(c)) = (term, context) {
                        output.push_str("- **");
                        output.push_str(t);
                        output.push_str("**: ");
                        output.push_str(c);
                        output.push('\n');
                    }
                }
            }
            output.push('\n');
        }
    }

    output
}

/// Extract tags from enrichment markdown
pub fn extract_tags(enrichment_md: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_tags_section = false;

    for line in enrichment_md.lines() {
        let trimmed = line.trim();
        
        if trimmed == "## Tags" {
            in_tags_section = true;
            continue;
        }
        
        if in_tags_section {
            if trimmed.starts_with("##") {
                break;
            }
            if trimmed.starts_with("- ") {
                let tag = trimmed[2..].trim().to_string();
                if !tag.is_empty() {
                    tags.push(tag);
                }
            }
        }
    }

    tags
}

/// Extract summary from enrichment markdown
pub fn extract_summary(enrichment_md: &str) -> Option<String> {
    let mut in_summary_section = false;
    let mut summary_lines = Vec::new();

    for line in enrichment_md.lines() {
        let trimmed = line.trim();
        
        if trimmed == "## Summary" {
            in_summary_section = true;
            continue;
        }
        
        if in_summary_section {
            if trimmed.starts_with("##") {
                break;
            }
            if !trimmed.is_empty() {
                summary_lines.push(trimmed);
            }
        }
    }

    if summary_lines.is_empty() {
        None
    } else {
        Some(summary_lines.join("\n"))
    }
}

/// Helper to read a subfile
async fn read_subfile(folder: &Path, filename: &str) -> Result<String, std::io::Error> {
    let path = folder.join(filename);
    tokio::fs::read_to_string(path).await
}

/// Assemble gem.md from subfiles
pub async fn assemble_gem_md(gem_folder: &Path, meta: &GemMeta) -> Result<String, String> {
    let mut doc = String::new();

    // Title heading
    doc.push_str(&format!("# {}\n\n", meta.title));

    // Metadata block
    doc.push_str(&format!("- **Source:** {}\n", meta.source_type));
    doc.push_str(&format!("- **URL:** {}\n", meta.source_url));
    if let Some(author) = &meta.author {
        doc.push_str(&format!("- **Author:** {}\n", author));
    }
    doc.push_str(&format!("- **Captured:** {}\n", meta.captured_at));

    // Read enrichment once, reuse for tags and summary
    let enrichment_content = read_subfile(gem_folder, "enrichment.md").await.ok();
    if let Some(ref enrichment) = enrichment_content {
        let tags = extract_tags(enrichment);
        if !tags.is_empty() {
            doc.push_str(&format!("- **Tags:** {}\n", tags.join(", ")));
        }
    }

    // Project (if assigned)
    if let Some(project_id) = &meta.project_id {
        doc.push_str(&format!("- **Project:** {}\n", project_id));
    }

    doc.push_str("\n");

    // Summary (from enrichment.md, already read above)
    if let Some(ref enrichment) = enrichment_content {
        if let Some(summary) = extract_summary(enrichment) {
            doc.push_str("## Summary\n\n");
            doc.push_str(&summary);
            doc.push_str("\n\n");
        }
    }

    // Content section
    if let Ok(content) = read_subfile(gem_folder, "content.md").await {
        doc.push_str("## Content\n\n");
        doc.push_str(&content);
        doc.push_str("\n\n");
    }

    // Transcript section
    if let Ok(transcript) = read_subfile(gem_folder, "transcript.md").await {
        doc.push_str(&transcript);
        doc.push_str("\n\n");
    }

    // Co-Pilot Analysis section
    if let Ok(copilot) = read_subfile(gem_folder, "copilot.md").await {
        doc.push_str("## Co-Pilot Analysis\n\n");
        doc.push_str(&copilot);
        doc.push_str("\n\n");
    }

    Ok(doc)
}
