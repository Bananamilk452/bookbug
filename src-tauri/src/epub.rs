use roxmltree::Document;
use serde_derive::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;
use std::sync::{Arc, Mutex};
use zip::ZipArchive;

#[derive(Serialize)]
struct Metadata {
    uuid: String,
    isbn: String,
    title: String,
    creator: String,
    language: String,
    date_modification: String,
    subject: String,
    description: String,
    rights: String,
    relation: String,
    date_publication: String,
    format: String,
    publisher: String,
}

#[derive(Serialize)]
pub struct EpubManifest {
    metadata: Metadata,
    spine: Vec<SpineItem>,
    manifest: HashMap<String, ManifestItem>,
}

#[derive(Serialize)]
struct ManifestItem {
    href: String,
    media_type: String,
}

#[derive(Serialize)]
struct SpineItem {
    idref: String,
}

pub struct EpubStore {
    pub files: HashMap<String, Vec<u8>>,
    pub manifest: EpubManifest,
}

impl EpubStore {
    pub fn load(epub_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(epub_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_file() {
                let name = entry.name().to_string();
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                files.insert(name, buf);
            }
        }

        let manifest = parse_epub(&files)?;

        Ok(Self { files, manifest })
    }
}

pub type EpubState = Arc<Mutex<Option<EpubStore>>>;

fn get_full_path_from_container_xml(contents: &str) -> Result<String, Box<dyn Error>> {
    let doc = Document::parse(contents)?;
    let node = doc.descendants().find(|n| n.has_tag_name("rootfile"));
    if let Some(node) = node {
        if let Some(full_path) = node.attribute("full-path") {
            Ok(full_path.to_string())
        } else {
            Err("Error: 'full-path' attribute not found in 'rootfile' element".into())
        }
    } else {
        Err("Error: 'rootfile' element not found in container.xml".into())
    }
}

fn extract_metadata_from_opf(contents: &str) -> Result<Metadata, Box<dyn Error>> {
    let doc = Document::parse(contents)?;
    let metadata_node = doc
        .descendants()
        .find(|n| n.has_tag_name("metadata"))
        .ok_or("Error: 'metadata' element not found in OPF file")?;

    let get_text = |tag: &str| {
        metadata_node
            .descendants()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string()
    };

    Ok(Metadata {
        uuid: get_text("identifier"),
        isbn: get_text("identifier"),
        title: get_text("title"),
        creator: get_text("creator"),
        language: get_text("language"),
        date_modification: get_text("date"),
        subject: get_text("subject"),
        description: get_text("description"),
        rights: get_text("rights"),
        relation: get_text("relation"),
        date_publication: get_text("date"),
        format: get_text("format"),
        publisher: get_text("publisher"),
    })
}

fn extract_manifest_from_opf(
    contents: &str,
) -> Result<HashMap<String, ManifestItem>, Box<dyn Error>> {
    let doc = Document::parse(contents)?;
    let manifest_node = doc
        .descendants()
        .find(|n| n.has_tag_name("manifest"))
        .ok_or("Error: 'manifest' element not found in OPF file")?;

    let mut manifest = HashMap::new();
    for item in manifest_node.children().filter(|n| n.has_tag_name("item")) {
        if let (Some(id), Some(href), Some(media_type)) = (
            item.attribute("id"),
            item.attribute("href"),
            item.attribute("media-type"),
        ) {
            manifest.insert(
                id.to_string(),
                ManifestItem {
                    href: href.to_string(),
                    media_type: media_type.to_string(),
                },
            );
        }
    }
    Ok(manifest)
}

fn extract_spine_from_opf(contents: &str) -> Result<Vec<SpineItem>, Box<dyn Error>> {
    let doc = Document::parse(contents)?;
    let spine_node = doc
        .descendants()
        .find(|n| n.has_tag_name("spine"))
        .ok_or("Error: 'spine' element not found in OPF file")?;

    let mut spine = Vec::new();
    for itemref in spine_node.children().filter(|n| n.has_tag_name("itemref")) {
        if let Some(idref) = itemref.attribute("idref") {
            spine.push(SpineItem {
                idref: idref.to_string(),
            });
        }
    }
    Ok(spine)
}

pub fn parse_epub(files: &HashMap<String, Vec<u8>>) -> Result<EpubManifest, Box<dyn Error>> {
    let container = files
        .get("META-INF/container.xml")
        .ok_or("Error: 'META-INF/container.xml' not found in EPUB archive")?;

    let opf_path = get_full_path_from_container_xml(
        std::str::from_utf8(container)
            .map_err(|_| "Error: unable to convert container.xml contents to UTF-8")?,
    )?;
    let opf = std::str::from_utf8(&files[&opf_path])
        .map_err(|_| "Error: unable to convert OPF contents to UTF-8")?;

    let metadata = extract_metadata_from_opf(opf)?;
    let manifest = extract_manifest_from_opf(opf)?;
    let spine = extract_spine_from_opf(opf)?;
    let epub_manifest = EpubManifest {
        metadata,
        manifest,
        spine,
    };

    Ok(epub_manifest)
}
