use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use roxmltree::{Document, Node};
use xml::{EmitterConfig, EventWriter};
use xml::writer::XmlEvent;

pub fn patch(original: &Path, patch: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let patch_content = fs::read_to_string(patch)?;
    let patch_doc = Document::parse(&patch_content)
        .context("Failed to parse patch XML")?;
    let mut patch_index = build_index(&patch_doc);

    let mut original_content = fs::read_to_string(original)
        .context("Failed to parse original XML")?;
    let needs_fix = original.file_name() == Some(OsStr::new("Facts.xml"));

    if needs_fix {
        original_content = original_content.replace("&&", "&amp;&amp;");
    }

    let original_doc = Document::parse(&original_content)?;

    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(Vec::with_capacity(original_content.len()));

    if let Some(first) = original_doc.root().first_element_child() {
        merge_to(&mut writer, first, &mut patch_index)
            .with_context(|| format!("Failed to patch {} with {}", original.display(), patch.display()))?;
        if needs_fix {
            let content = String::from_utf8(writer.into_inner())?.replace("&amp;&amp;", "&&");
            fs::write(output, content).context("Failed to write to output file")?
        } else {
            fs::write(output, writer.into_inner()).context("Failed to write to output file")?
        }
    }

    Ok(())
}

type NodeIndex<'doc, 'input> = HashMap<String, HashMap<String, Node<'doc, 'input>>>;

/// TODO, don't index children of nodes with an id
fn build_index<'doc, 'input: 'doc>(
    doc: &'doc Document<'input>
) -> NodeIndex<'doc, 'input> {
    let mut index = HashMap::new();
    for node in doc.descendants() {
        if node.is_element() {
            if let Some(id) = get_id(&node, true) {
                let path = get_node_path(&node, false);
                let map = index.entry(path.clone()).or_insert_with(HashMap::new);
                map.insert(id, node);
            }
        }
    }
    index
}

fn get_id(node: &Node, is_index: bool) -> Option<String> {
    if !node.is_element() {
        return None;
    }

    match node.tag_name().name() {
        "paper" => {
            let id = node.attribute("id").unwrap();
            let nation = node.attribute("nation").unwrap_or("");
            Some(format!("pa#{id}#{nation}"))
        }
        "purpose" => {
            let val = node.attribute("val").unwrap().to_string();
            Some(format!("pr#{val}"))
        }
        &_ => {
            let name = node.tag_name().name();
            if node.attributes().count() == 0 {
                if !is_index {
                    Some(format!("{name}#override"))
                } else {
                    None
                }
            } else if let Some(id) = node.attribute("id") {
                let id = id.to_string();
                Some(format!("{name}#{id}"))
            } else {
                let attrs = node.attributes()
                    .map(|a| format!("{}={}", a.name(), a.value()))
                    .collect::<Vec<_>>()
                    .join(",");
                Some(format!("{}[{}]", name, attrs))
            }
        }
    }
}

fn get_node_path(node: &Node, incl_self: bool) -> String {
    let mut path = node.ancestors()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect::<Vec<_>>()
        .join("/");
    if !incl_self {
        if let Some(index) = path.find('/') {
            // substring everything after the first slash
            path = path.split_at(index + 1).1.to_string();
        } else {
            // we're at root, so just return an empty string
            path = String::new();
        }
    }

    path
}

fn merge_to(
    writer: &mut EventWriter<Vec<u8>>,
    node: Node,
    patch_index: &mut NodeIndex,
) -> anyhow::Result<()> {
    let id = get_id(&node, false);

    // if the node has an id, check if there are any patches for it
    if let Some(id) = id {
        let path = get_node_path(&node, false);
        // check if this node has a patch
        if let Some(path_map) = patch_index.get_mut(&path) {
            if let Some(patched_node) = path_map.get(&id) {
                // this node is patched, so write the patched version and don't process the child nodes
                write_node(writer, patched_node)
                    .context(format!("Failed to write patched node: {}", id))?;
                path_map.remove(&id);
                if path_map.is_empty() {
                    patch_index.remove(&path);
                }
                return Ok(());
            }
        }

        if node.attributes().len() > 0 {
            // this node has an id, but no patch, so write the original node and process the child nodes
            write_node(writer, &node)
                .context(format!("Failed to write node: {}", id))?;
            return Ok(());
        }
    }

    // this node doesn't have an id, so write the tag and process the child nodes
    let path = get_node_path(&node, true);

    writer.write(XmlEvent::start_element(node.tag_name().name()))?;

    for child in node.children() {
        if child.is_element() {
            merge_to(writer, child, patch_index)?;
        } else if child.is_text() {
            writer.write(XmlEvent::characters(child.text().unwrap()))?;
        } else if child.is_comment() {
            writer.write(XmlEvent::comment(child.text().unwrap()))?;
        }
    }

    // write any remaining nodes that were newly added with the patch at this path
    if let Some(path_map) = patch_index.get_mut(&path) {
        for new_node in path_map.values() {
            write_node(writer, new_node)
                .context(format!("Failed to write new patched nodes at: {}", path))?;
        }
        path_map.clear();
        patch_index.remove(&path);
    }

    // close the tag
    writer.write(XmlEvent::end_element())?;

    Ok(())
}


fn write_node(writer: &mut EventWriter<Vec<u8>>, node: &Node) -> anyhow::Result<()> {
    if node.is_element() {
        let mut element = XmlEvent::start_element(node.tag_name().name());
        for attr in node.attributes() {
            if attr.name() == "id" && attr.value() == "override" {
                continue;
            }
            element = element.attr(attr.name(), attr.value());
        }
        writer.write(element)?;

        for child in node.children() {
            if child.is_element() {
                write_node(writer, &child)?;
            } else if child.is_text() {
                writer.write(XmlEvent::characters(child.text().unwrap()))?;
            } else if child.is_comment() {
                writer.write(XmlEvent::comment(child.text().unwrap()))?;
            }
        }

        writer.write(XmlEvent::end_element())?;
    } else if node.is_text() {
        writer.write(XmlEvent::characters(node.text().unwrap()))?;
    }

    Ok(())
}